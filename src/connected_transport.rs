//! Single-peer KNX connected transport for standard-length individual TPDUs.
//!
//! Supply only individual indications addressed to the local device. Send each
//! returned request through the network layer with link acknowledgement requested.
//! Control requests use system priority. Drive the endpoint with a monotonic
//! clock and service [`Endpoint::deadline`] before submitting more events.

use std::{fmt, time::Duration};

const ACK: Duration = Duration::from_secs(3);
const IDLE: Duration = Duration::from_secs(6);

/// KNX network priority, retained for every retransmission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Priority {
    System,
    Normal,
    Urgent,
    Low,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Connecting,
    Idle,
    AwaitingAck,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidTpdu,
    InvalidApdu,
    Busy,
    ClockReversed,
    TimerDue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidTpdu => "Invalid standard-length connected transport PDU.",
            Self::InvalidApdu => "An APDU requires 2–16 octets and six clear transport bits.",
            Self::Busy => "The connection is not ready for this request.",
            Self::ClockReversed => "The transport clock moved backwards.",
            Self::TimerDue => "Poll the due transport deadline before submitting another event.",
        })
    }
}
impl std::error::Error for Error {}

/// An individual network request. The TPDU includes the transport bits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub destination: u16,
    pub priority: Priority,
    pub tpdu: Vec<u8>,
}
impl Request {
    fn control(destination: u16, code: u8) -> Self {
        Self {
            destination,
            priority: Priority::System,
            tpdu: vec![code],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseReason {
    Remote,
    LinkFailure,
    Protocol,
    AckTimeout,
    IdleTimeout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notification {
    Connected,
    /// APDU with the six transport bits removed.
    Data {
        priority: Priority,
        apdu: Vec<u8>,
    },
    Acknowledged,
    Closed(CloseReason),
}
/// Dispatch the network request before delivering the accompanying notification.
#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct Effects {
    pub request: Option<Request>,
    pub notification: Option<Notification>,
}

#[derive(Debug)]
struct Pending {
    request: Request,
    retries: u8,
    due: Duration,
}

/// One connection lifetime. Construct a fresh endpoint for a new exchange.
#[derive(Debug)]
pub struct Endpoint {
    peer: u16,
    state: State,
    sent: u8,
    received: u8,
    pending: Option<Pending>,
    now: Duration,
    idle_due: Duration,
}
impl Endpoint {
    fn new(peer: u16, state: State, now: Duration) -> Self {
        Self {
            peer,
            state,
            sent: 0,
            received: 0,
            pending: None,
            now,
            idle_due: now.saturating_add(IDLE),
        }
    }
    /// Initiate a connection. Pass its network confirmation to [`Self::confirm_connect`].
    pub fn connect(peer: u16, now: Duration) -> (Self, Request) {
        (
            Self::new(peer, State::Connecting, now),
            Request::control(peer, 0x80),
        )
    }
    /// Accept an incoming T_Connect indication from an application-selected peer.
    pub fn accept(peer: u16, tpdu: &[u8], now: Duration) -> Result<Self, Error> {
        if tpdu != [0x80] {
            return Err(Error::InvalidTpdu);
        }
        Ok(Self::new(peer, State::Idle, now))
    }
    pub fn state(&self) -> State {
        self.state
    }
    pub fn deadline(&self) -> Option<Duration> {
        (self.state != State::Closed).then(|| {
            self.pending
                .as_ref()
                .map_or(self.idle_due, |p| p.due.min(self.idle_due))
        })
    }
    fn advance(&mut self, now: Duration) -> Result<(), Error> {
        if now < self.now {
            return Err(Error::ClockReversed);
        }
        self.now = now;
        Ok(())
    }
    fn ready(&mut self, now: Duration) -> Result<(), Error> {
        self.advance(now)?;
        if self.deadline().is_some_and(|due| due <= now) {
            return Err(Error::TimerDue);
        }
        Ok(())
    }
    fn close(&mut self, reason: CloseReason, transmit: bool) -> Effects {
        self.state = State::Closed;
        self.pending = None;
        Effects {
            request: transmit.then(|| Request::control(self.peer, 0x81)),
            notification: Some(Notification::Closed(reason)),
        }
    }
    /// Supply the local network confirmation for this endpoint's connect request.
    pub fn confirm_connect(&mut self, ok: bool, now: Duration) -> Result<Effects, Error> {
        self.ready(now)?;
        if self.state != State::Connecting {
            return Err(Error::Busy);
        }
        Ok(if ok {
            self.state = State::Idle;
            Effects {
                notification: Some(Notification::Connected),
                ..Effects::default()
            }
        } else {
            self.close(CloseReason::LinkFailure, false)
        })
    }
    /// Release locally. A network confirmation for the returned request tells the
    /// caller whether the disconnect reached the link.
    pub fn disconnect(&mut self) -> Option<Request> {
        if self.state == State::Closed {
            return None;
        }
        self.state = State::Closed;
        self.pending = None;
        Some(Request::control(self.peer, 0x81))
    }
    /// Submit one APDU. Wait for acknowledgement before submitting the next.
    pub fn send(
        &mut self,
        apdu: &[u8],
        priority: Priority,
        now: Duration,
    ) -> Result<Request, Error> {
        self.ready(now)?;
        if self.state != State::Idle {
            return Err(Error::Busy);
        }
        if !(2..=16).contains(&apdu.len()) || apdu[0] > 3 {
            return Err(Error::InvalidApdu);
        }
        let mut tpdu = apdu.to_vec();
        tpdu[0] |= 0x40 | self.sent << 2;
        let request = Request {
            destination: self.peer,
            priority,
            tpdu,
        };
        self.state = State::AwaitingAck;
        self.idle_due = now.saturating_add(IDLE);
        self.pending = Some(Pending {
            request: request.clone(),
            retries: 0,
            due: now.saturating_add(ACK),
        });
        Ok(request)
    }
    fn retry(&mut self, now: Duration, exhausted: CloseReason) -> Effects {
        let Some(pending) = self.pending.as_mut() else {
            return Effects::default();
        };
        if pending.retries == 3 {
            return self.close(exhausted, true);
        }
        pending.retries += 1;
        pending.due = now.saturating_add(ACK);
        self.idle_due = now.saturating_add(IDLE);
        Effects {
            request: Some(pending.request.clone()),
            notification: None,
        }
    }
    /// Advance timers. An application that misses the entire idle window closes
    /// the expired connection instead of replaying a stale operation.
    pub fn poll(&mut self, now: Duration) -> Result<Effects, Error> {
        self.advance(now)?;
        if self.state == State::Closed {
            return Ok(Effects::default());
        }
        if now >= self.idle_due {
            return Ok(self.close(CloseReason::IdleTimeout, true));
        }
        if self.pending.as_ref().is_some_and(|p| now >= p.due) {
            return Ok(self.retry(now, CloseReason::AckTimeout));
        }
        Ok(Effects::default())
    }
    /// Process an individual TPDU. Peer addresses identify a connection and do
    /// not authenticate its sender.
    pub fn receive(
        &mut self,
        source: u16,
        priority: Priority,
        tpdu: &[u8],
        now: Duration,
    ) -> Result<Effects, Error> {
        self.ready(now)?;
        let code = *tpdu.first().ok_or(Error::InvalidTpdu)?;
        let data = code >> 6 == 1;
        let control = matches!(code, 0x80 | 0x81) || matches!(code & 0xc3, 0xc2 | 0xc3);
        if !(data && (2..=16).contains(&tpdu.len()) || control && tpdu.len() == 1) {
            return Err(Error::InvalidTpdu);
        }
        if self.state == State::Closed {
            return Ok(Effects::default());
        }
        if source != self.peer {
            return Ok(Effects {
                request: (code == 0x80 || self.state == State::Connecting && code != 0x81)
                    .then(|| Request::control(source, 0x81)),
                notification: None,
            });
        }
        if code == 0x80 {
            return Ok(Effects::default());
        }
        if code == 0x81 {
            return Ok(self.close(CloseReason::Remote, false));
        }
        let sequence = code >> 2 & 15;
        if data {
            let duplicate = sequence == (self.received.wrapping_sub(1) & 15);
            if self.state == State::Connecting && !duplicate {
                return Ok(self.close(CloseReason::Protocol, true));
            }
            let expected = sequence == self.received;
            let ack = if expected || duplicate { 0xc2 } else { 0xc3 };
            self.idle_due = now.saturating_add(IDLE);
            let notification = expected.then(|| {
                self.received = (self.received + 1) & 15;
                let mut apdu = tpdu.to_vec();
                apdu[0] &= 3;
                Notification::Data { priority, apdu }
            });
            return Ok(Effects {
                request: Some(Request::control(source, ack | sequence << 2)),
                notification,
            });
        }
        let nak = code & 1 != 0;
        if self.state == State::Connecting {
            return Ok(self.close(CloseReason::Protocol, true));
        }
        if self.state == State::Idle {
            return Ok(if nak && sequence == self.sent {
                self.close(CloseReason::Protocol, true)
            } else {
                Effects::default()
            });
        }
        if sequence != self.sent {
            return Ok(self.close(CloseReason::Protocol, true));
        }
        if nak {
            return Ok(self.retry(now, CloseReason::Protocol));
        }
        self.pending = None;
        self.sent = (self.sent + 1) & 15;
        self.state = State::Idle;
        self.idle_due = now.saturating_add(IDLE);
        Ok(Effects {
            request: None,
            notification: Some(Notification::Acknowledged),
        })
    }
}

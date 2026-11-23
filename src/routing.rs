//! Plain KNXnet/IP group routing on a selected IPv4 multicast interface.
//!
//! Deliver the actual ingress interface and the UDP source endpoint to the
//! receiver. The host joins and leaves the group, controls its socket queue,
//! and supplies a local sending endpoint if local echo suppression is required.

use crate::GroupAddress;
use crate::ip_core::{IpError, IpFrame};
use crate::telegram::{GroupService, GroupTelegram, LinkPrimitive, TelegramError};
use std::{
    fmt,
    hash::{BuildHasher, RandomState},
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    time::{Duration, Instant},
};

#[cfg(all(target_os = "linux", feature = "linux-routing"))]
mod linux;
#[cfg(all(target_os = "linux", feature = "linux-routing"))]
pub use linux::{LinuxRoutingSocket, RoutingDatagram};

/// A single standard routing indication or a flow-control report.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RoutingEvent<'a> {
    Indication(GroupTelegram<'a>),
    Lost {
        device_state: u8,
        messages: u16,
    },
    Busy {
        device_state: u8,
        wait: Duration,
        control: u16,
    },
}

/// Reason a datagram could not be interpreted as a receive-side routing event.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RoutingError {
    Ip(IpError),
    Telegram(TelegramError),
    Body,
    UnsupportedService,
}

impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ip(e) => e.fmt(f),
            Self::Telegram(e) => e.fmt(f),
            Self::Body => f.write_str("Check the routing body length and cEMI primitive."),
            Self::UnsupportedService => {
                f.write_str("Use a plain routing indication, loss or busy report.")
            }
        }
    }
}
impl std::error::Error for RoutingError {}

impl<'a> RoutingEvent<'a> {
    /// Validate one routing datagram. Reports carry the router's advertised state.
    pub fn parse(datagram: &'a [u8]) -> Result<Self, RoutingError> {
        let frame = IpFrame::parse(datagram).map_err(RoutingError::Ip)?;
        match (frame.service, frame.body) {
            (0x0530, body) => {
                let telegram = GroupTelegram::parse(body).map_err(RoutingError::Telegram)?;
                if telegram.primitive != LinkPrimitive::Indication {
                    return Err(RoutingError::Body);
                }
                Ok(Self::Indication(telegram))
            }
            (0x0531, [4, state, h, l]) => Ok(Self::Lost {
                device_state: *state,
                messages: u16::from_be_bytes([*h, *l]),
            }),
            (0x0532, [6, state, 0, wait @ 20..=100, ch, cl]) => Ok(Self::Busy {
                device_state: *state,
                wait: Duration::from_millis((*wait).into()),
                control: u16::from_be_bytes([*ch, *cl]),
            }),
            (0x0531 | 0x0532, _) => Err(RoutingError::Body),
            _ => Err(RoutingError::UnsupportedService),
        }
    }
}

/// A synchronous receive filter. Instantiate one per selected interface.
/// Events are returned in caller delivery order with no internal queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoutingReceiver {
    interface: Ipv4Addr,
    local_sender: Option<SocketAddrV4>,
}

impl RoutingReceiver {
    pub fn new(interface: Ipv4Addr, local_sender: Option<SocketAddrV4>) -> Result<Self, IpError> {
        if interface.is_unspecified() || interface.is_broadcast() || interface.is_multicast() {
            return Err(IpError::Endpoint);
        }
        Ok(Self {
            interface,
            local_sender,
        })
    }
    /// Other interfaces and the configured exact local sender are ignored.
    pub fn receive<'a>(
        &self,
        ingress: Ipv4Addr,
        sender: SocketAddrV4,
        datagram: &'a [u8],
    ) -> Result<Option<RoutingEvent<'a>>, RoutingError> {
        if ingress != self.interface || Some(sender) == self.local_sender {
            return Ok(None);
        }
        RoutingEvent::parse(datagram).map(Some)
    }
}

/// An IPv4 multicast membership tied to a socket and one selected interface.
/// Obtain ingress metadata from the transport when sharing sockets across interfaces.
#[derive(Debug)]
pub struct RoutingMembership<'a> {
    socket: &'a std::net::UdpSocket,
    group: Ipv4Addr,
    interface: Ipv4Addr,
}

impl<'a> RoutingMembership<'a> {
    /// Join a commissioned multicast group. Dropping this handle leaves it.
    pub fn join(
        socket: &'a std::net::UdpSocket,
        group: Ipv4Addr,
        interface: Ipv4Addr,
    ) -> std::io::Result<Self> {
        if !group.is_multicast() || RoutingReceiver::new(interface, None).is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                IpError::Endpoint,
            ));
        }
        socket.join_multicast_v4(&group, &interface)?;
        Ok(Self {
            socket,
            group,
            interface,
        })
    }
}

impl Drop for RoutingMembership<'_> {
    fn drop(&mut self) {
        let _ = self.socket.leave_multicast_v4(&self.group, &self.interface);
    }
}

#[derive(Debug)]
pub(crate) struct FlowControl {
    next_send: Instant,
    wait_until: Instant,
    resume: Instant,
    slow_until: Instant,
    last_busy: Option<Instant>,
    count: u32,
}

impl FlowControl {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            next_send: now,
            wait_until: now,
            resume: now,
            slow_until: now,
            last_busy: None,
            count: 0,
        }
    }

    pub(crate) fn ready_at(&self) -> Instant {
        self.next_send.max(self.resume)
    }

    pub(crate) fn busy(&mut self, now: Instant, wait: Duration, sample: u32) {
        let elapsed = now.saturating_duration_since(self.slow_until).as_millis() / 5;
        self.count = self
            .count
            .saturating_sub(u32::try_from(elapsed).unwrap_or(u32::MAX));
        if self
            .last_busy
            .is_none_or(|previous| now.duration_since(previous) >= Duration::from_millis(10))
        {
            self.count = self.count.saturating_add(1);
        }
        self.last_busy = Some(now);
        self.wait_until = self.wait_until.max(now + wait);
        let max_jitter_ms = u64::from(self.count) * 50;
        let jitter_ms = u128::from(max_jitter_ms) * u128::from(sample) / u128::from(u32::MAX);
        self.resume = self
            .resume
            .max(self.wait_until + Duration::from_millis(jitter_ms as u64));
        self.slow_until = self.resume + Duration::from_millis(u64::from(self.count) * 100);
    }

    pub(crate) fn sent(&mut self, now: Instant) {
        // Uniform spacing satisfies both the 50/s limit and the 5 ms busy recovery spacing.
        self.next_send = now + Duration::from_millis(20);
    }
}

/// Outcome of a queue-free send. UDP acceptance provides no bus delivery confirmation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SendOutcome {
    Sent,
    /// No datagram was sent. Recheck the sender at this instant after processing new busy reports.
    RetryAt(Instant),
}

/// A single application's group sender with routing-busy flow control.
/// Share one instance for a commissioned interface and routing group.
#[derive(Debug)]
pub struct RoutingSender {
    socket: UdpSocket,
    target: SocketAddrV4,
    address: u16,
    receiver: RoutingReceiver,
    flow: FlowControl,
    jitter: RandomState,
}

impl RoutingSender {
    /// Bind the socket to a concrete local IPv4 address and select its outgoing multicast
    /// interface before passing it here. The individual address belongs to this application.
    pub fn new(socket: UdpSocket, group: Ipv4Addr, address: u16) -> io::Result<Self> {
        let peer_error = || io::Error::new(io::ErrorKind::InvalidInput, IpError::Endpoint);
        let SocketAddr::V4(local) = socket.local_addr()? else {
            return Err(peer_error());
        };
        if !group.is_multicast() || socket.peer_addr().is_ok() {
            return Err(peer_error());
        }
        let receiver = RoutingReceiver::new(*local.ip(), Some(local)).map_err(|_| peer_error())?;
        Ok(Self {
            socket,
            target: SocketAddrV4::new(group, 3671),
            address,
            receiver,
            flow: FlowControl::new(Instant::now()),
            jitter: RandomState::new(),
        })
    }

    /// Feed datagrams received for this routing group, supplying their actual ingress interface.
    /// A valid busy report updates the sender before its event is returned to the caller.
    pub fn receive<'a>(
        &mut self,
        ingress: Ipv4Addr,
        sender: SocketAddrV4,
        datagram: &'a [u8],
    ) -> Result<Option<RoutingEvent<'a>>, RoutingError> {
        let event = self.receiver.receive(ingress, sender, datagram)?;
        if let Some(RoutingEvent::Busy { wait, .. }) = event {
            let now = Instant::now();
            self.flow.busy(now, wait, self.jitter.hash_one(now) as u32);
        }
        Ok(event)
    }

    /// Earliest time to send, accounting for the busy reports processed so far.
    pub fn ready_at(&self) -> Instant {
        self.flow.ready_at()
    }

    /// Originate one group service on the configured multicast group.
    /// A busy or rate-limited caller receives a deadline and retains ownership of its message.
    pub fn send(
        &mut self,
        destination: GroupAddress,
        service: GroupService<'_>,
    ) -> io::Result<SendOutcome> {
        let mut packet = [0; 6 + GroupTelegram::MAX_ENCODED_LEN];
        let packet = group_indication(self.address, destination, service, &mut packet)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let ready = self.ready_at();
        if Instant::now() < ready {
            return Ok(SendOutcome::RetryAt(ready));
        }
        self.socket.send_to(packet, self.target)?;
        self.flow.sent(Instant::now());
        Ok(SendOutcome::Sent)
    }
}

pub(crate) fn group_indication<'a>(
    source: u16,
    destination: GroupAddress,
    service: GroupService<'_>,
    packet: &'a mut [u8; 6 + GroupTelegram::MAX_ENCODED_LEN],
) -> Result<&'a [u8], TelegramError> {
    let mut telegram = GroupTelegram::request(source, destination, service);
    telegram.primitive = LinkPrimitive::Indication;
    let size = 6 + telegram.encode(&mut packet[6..])?;
    let [high, low] = (size as u16).to_be_bytes();
    packet[..6].copy_from_slice(&[6, 16, 5, 0x30, high, low]);
    Ok(&packet[..size])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_group_value_fits_a_routing_datagram() {
        let localhost = Ipv4Addr::LOCALHOST;
        let rx = UdpSocket::bind((localhost, 0)).unwrap();
        rx.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let tx = UdpSocket::bind((localhost, 0)).unwrap();
        let mut sender = RoutingSender::new(tx, Ipv4Addr::new(239, 255, 0, 1), 0x1101).unwrap();
        // Keep the wire test on loopback and let the OS assign an isolated port.
        let SocketAddr::V4(peer) = rx.local_addr().unwrap() else {
            unreachable!()
        };
        sender.target = peer;
        let service = GroupService::Write(crate::telegram::GroupData::Bytes(&[7; 253]));
        assert_eq!(
            sender.send("1/2/1".parse().unwrap(), service).unwrap(),
            SendOutcome::Sent
        );
        let mut packet = [0; 270];
        let size = rx.recv(&mut packet).unwrap();
        assert_eq!(size, 270);
        assert_eq!(&packet[..6], &[6, 16, 5, 0x30, 1, 14]);
        assert!(
            matches!(RoutingEvent::parse(&packet), Ok(RoutingEvent::Indication(t))
            if t.service == service && t.source == 0x1101)
        );
    }

    #[test]
    fn busy_bursts_decay_and_wait_overlap_use_monotonic_time() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut flow = FlowControl::new(start);
        flow.sent(start);
        assert_eq!(flow.ready_at(), at(20));
        flow.busy(at(0), Duration::from_millis(100), u32::MAX);
        assert_eq!(flow.ready_at(), at(150));
        flow.busy(at(9), Duration::from_millis(20), 0);
        assert_eq!((flow.count, flow.ready_at()), (1, at(150)));
        flow.busy(at(19), Duration::from_millis(100), u32::MAX);
        assert_eq!(
            (flow.count, flow.ready_at(), flow.slow_until),
            (2, at(219), at(419))
        );
        flow.busy(at(424), Duration::from_millis(20), 0);
        assert_eq!((flow.count, flow.ready_at()), (2, at(444)));
        flow.busy(at(1000), Duration::from_millis(20), 0);
        assert_eq!((flow.count, flow.ready_at()), (1, at(1020)));
    }
}

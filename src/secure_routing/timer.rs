//! KNXnet/IP multicast time synchronization (ISO 22510:2019, 5.7.2.2).

use super::{MAX_TIMER, SecureRoutingError};
use crate::{ip_core::IpFrame, ip_secure_crypto};
use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Commissioned secure multicast membership and socket used to originate notifications.
#[derive(Debug, Clone, Copy)]
pub struct TimerConfig {
    pub interface_index: u32,
    pub multicast_group: Ipv4Addr,
    /// Actual source socket of locally transmitted notifications, for echo suppression.
    pub local_source: SocketAddrV4,
    /// Exclusive local serial for the timer participant.
    pub serial: [u8; 6],
    /// PID_MULTICAST_LATENCY_TOLERANCE, in milliseconds.
    pub latency_ms: u16,
    /// PID_SYNC_LATENCY_FRACTION as the raw one-octet scaling value (factory 0x1a).
    pub sync_fraction: u8,
}

/// How the participant entered this secure domain.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimerStart {
    /// Apply the randomized initial delay of up to ten seconds.
    PowerUp,
    /// Issue the initial notification immediately on joining or when data is needed.
    Immediate,
}

/// First use requires positive commissioning evidence for this backbone key.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimerOrigin {
    NewKey,
    /// Last durably reserved upper bound for this key, even after restoration.
    ReservedThrough(u64),
}

/// Exclusive, rollback-resistant storage for this installation, serial and backbone key.
/// Missing or corrupt records for previously used keys must return an error.
/// Atomically commit and flush a strictly higher ceiling before returning success.
pub trait TimerStore {
    fn origin(&mut self) -> io::Result<TimerOrigin>;
    fn reserve_through(&mut self, ceiling_ms: u64) -> io::Result<()>;
}

/// Drive notifications from the host's event loop. The state machine owns no socket or thread.
pub struct GroupClock<S> {
    config: TimerConfig,
    key: Zeroizing<[u8; 16]>,
    store: S,
    ceiling: u64,
    storage_failed: bool,
    base: u64,
    anchor: Instant,
    last_now: Instant,
    next: Instant,
    keeper: bool,
    update: Option<([u8; 6], u16)>,
    acquisition_deadline: Option<Instant>,
    challenge: Option<(u64, u16)>,
    authentic: bool,
}

fn invalid(error: SecureRoutingError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

fn random_in(min: u32, max: u32) -> io::Result<u32> {
    let width = max - min + 1;
    let limit = u32::MAX - u32::MAX % width;
    loop {
        let n = getrandom::u32()
            .map_err(|_| io::Error::other("Multicast timer randomness unavailable."))?;
        if n < limit {
            return Ok(min + n % width);
        }
    }
}

fn delay(ms: u32) -> Duration {
    Duration::from_millis(ms.into())
}

impl<S: TimerStore> GroupClock<S> {
    pub fn new(
        config: TimerConfig,
        backbone_key: [u8; 16],
        mut store: S,
        now: Instant,
        start: TimerStart,
    ) -> io::Result<Self> {
        let local = config.local_source.ip();
        if config.interface_index == 0
            || !config.multicast_group.is_multicast()
            || local.is_unspecified()
            || local.is_multicast()
            || local.is_broadcast()
            || config.local_source.port() == 0
            || !(1..=8000).contains(&config.latency_ms)
        {
            return Err(invalid(SecureRoutingError::Configuration));
        }
        let base = match store.origin()? {
            TimerOrigin::NewKey => 0,
            TimerOrigin::ReservedThrough(previous) => previous
                .checked_add(1)
                .filter(|v| *v <= MAX_TIMER)
                .ok_or_else(|| invalid(SecureRoutingError::Clock))?,
        };
        let next = now
            + delay(match start {
                TimerStart::PowerUp => random_in(0, 10_000)?,
                TimerStart::Immediate => 0,
            });
        let ceiling = (base + 60_000).min(MAX_TIMER);
        store.reserve_through(ceiling)?;
        Ok(Self {
            config,
            key: Zeroizing::new(backbone_key),
            store,
            ceiling,
            storage_failed: false,
            base,
            anchor: now,
            last_now: now,
            next,
            keeper: false,
            update: None,
            acquisition_deadline: None,
            challenge: None,
            authentic: false,
        })
    }

    fn reserve(&mut self, value: u64) -> io::Result<()> {
        if value > MAX_TIMER {
            return Err(invalid(SecureRoutingError::Clock));
        }
        if value > self.ceiling {
            let ceiling = (value + 60_000).min(MAX_TIMER);
            if let Err(error) = self.store.reserve_through(ceiling) {
                self.storage_failed = true;
                return Err(error);
            }
            self.ceiling = ceiling;
        }
        Ok(())
    }

    fn time(&mut self, now: Instant) -> io::Result<u64> {
        if self.storage_failed {
            return Err(io::Error::other(
                "Restore the durable timer store before resuming.",
            ));
        }
        if now < self.last_now {
            return Err(invalid(SecureRoutingError::Clock));
        }
        let elapsed = u64::try_from(now.duration_since(self.anchor).as_millis())
            .map_err(|_| invalid(SecureRoutingError::Clock))?;
        let value = self
            .base
            .checked_add(elapsed)
            .ok_or_else(|| invalid(SecureRoutingError::Clock))?;
        self.reserve(value)?;
        self.last_now = now;
        if self.acquisition_deadline.is_some_and(|t| now >= t) {
            self.authentic = true;
        }
        Ok(value)
    }

    fn sync_ms(&self) -> u32 {
        u32::from(self.config.latency_ms) * u32::from(self.config.sync_fraction) / 255
    }

    fn period(&self, keeper: bool) -> io::Result<Duration> {
        let sync = self.sync_ms();
        let (min, max) = if keeper {
            (10_000, 10_000 + 3 * sync)
        } else {
            (10_000 + 4 * sync, 10_000 + 14 * sync)
        };
        random_in(min, max).map(delay)
    }

    fn start_acquisition(&mut self, now: Instant) {
        let window = 100 + 12 * self.sync_ms() + 2 * u32::from(self.config.latency_ms);
        self.acquisition_deadline.get_or_insert(now + delay(window));
    }

    /// Earliest pending notification or acquisition-completion deadline.
    pub fn deadline(&self) -> Instant {
        if self.authentic {
            self.next
        } else {
            self.acquisition_deadline
                .map_or(self.next, |t| t.min(self.next))
        }
    }

    /// Time for a secure routing sender or receiver. `None` means acquisition is in progress.
    pub fn sample(&mut self, now: Instant) -> io::Result<Option<u64>> {
        let value = self.time(now)?;
        Ok(self.authentic.then_some(value))
    }

    /// Bind an application receiver to the acquired timer and backbone. Its startup
    /// replay fence excludes records from before this receiver was attached.
    pub fn receiver(
        &mut self,
        now: Instant,
        serial: [u8; 6],
    ) -> io::Result<super::SecureRoutingReceiver> {
        let value = self
            .sample(now)?
            .ok_or_else(|| invalid(SecureRoutingError::Clock))?;
        super::SecureRoutingReceiver::new(
            self.config.interface_index,
            self.config.multicast_group,
            serial,
            *self.key,
            self.config.latency_ms,
            value,
        )
        .map_err(invalid)
    }

    /// Submit one due Timer Notify through the configured source socket. An I/O error
    /// retains the pending notification. The callback must return success only on submission.
    pub fn poll(
        &mut self,
        now: Instant,
        submit: impl FnOnce(SocketAddrV4, &[u8; 36]) -> io::Result<()>,
    ) -> io::Result<bool> {
        let time = self.time(now)?;
        if now < self.next {
            return Ok(false);
        }
        let (serial, tag) = match self.update {
            Some(pair) => pair,
            None => (self.config.serial, random_in(0, u16::MAX.into())? as u16),
        };
        let mut frame = [0; 36];
        frame[..6].copy_from_slice(&[6, 16, 9, 0x55, 0, 36]);
        frame[6..12].copy_from_slice(&time.to_be_bytes()[2..]);
        frame[12..18].copy_from_slice(&serial);
        frame[18..20].copy_from_slice(&tag.to_be_bytes());
        let mac = ip_secure_crypto::timer_mac(&self.key, frame[..20].try_into().unwrap());
        frame[20..].copy_from_slice(&mac);
        let next = now + self.period(true)?;
        submit(SocketAddrV4::new(self.config.multicast_group, 3671), &frame)?;
        if self.update.is_none() && !self.authentic {
            self.challenge.get_or_insert((time, tag));
        }
        self.start_acquisition(now);
        self.update = None;
        self.keeper = true;
        self.next = next;
        Ok(true)
    }

    /// Authenticate one Timer Notify or session-zero Secure Wrapper on the actual routing
    /// endpoint. A returned timer allows this wrapper to proceed to its commissioned
    /// receiver, which applies sender binding, application decoding and duplicate checks.
    /// `None` covers notifications, local echo, acquisition and stale wrappers.
    pub fn observe(
        &mut self,
        now: Instant,
        interface_index: u32,
        destination: SocketAddrV4,
        source: SocketAddrV4,
        packet: &[u8],
    ) -> io::Result<Option<u64>> {
        if interface_index != self.config.interface_index
            || destination != SocketAddrV4::new(self.config.multicast_group, 3671)
        {
            return Err(invalid(SecureRoutingError::Binding));
        }
        if source == self.config.local_source {
            return Ok(None);
        }
        let frame =
            IpFrame::parse(packet).map_err(|_| invalid(SecureRoutingError::Authentication))?;
        let notify = frame.service == 0x0955;
        let fields = match frame.service {
            0x0955 if packet.len() == 36 => {
                let mac = ip_secure_crypto::timer_mac(&self.key, packet[..20].try_into().unwrap());
                if !bool::from(mac.ct_eq(&packet[20..])) {
                    return Err(invalid(SecureRoutingError::Authentication));
                }
                &packet[6..20]
            }
            0x0950 if packet.get(6..8) == Some(&[0, 0]) => {
                ip_secure_crypto::open(&self.key, packet)
                    .ok_or_else(|| invalid(SecureRoutingError::Authentication))?;
                &packet[8..22]
            }
            _ => return Err(invalid(SecureRoutingError::Authentication)),
        };
        let mut bytes = [0; 8];
        bytes[2..].copy_from_slice(&fields[..6]);
        let received = u64::from_be_bytes(bytes);
        let serial = fields[6..12].try_into().unwrap();
        let tag = u16::from_be_bytes(fields[12..].try_into().unwrap());
        let local = self.time(now)?;
        let behind = local.saturating_sub(received);
        let future = received > local;
        if future {
            self.reserve(received)?;
            self.base = received;
            self.anchor = now;
        }
        self.start_acquisition(now);
        if behind >= u64::from(self.config.latency_ms) {
            if self.update.is_none() {
                let sync = self.sync_ms();
                let (min, max) = if self.keeper {
                    (100, 100 + sync)
                } else {
                    (100 + 2 * sync, 100 + 12 * sync)
                };
                self.next = now + delay(random_in(min, max)?);
                self.update = Some((serial, tag));
            }
            return Ok(None);
        }
        if notify
            && serial == self.config.serial
            && self
                .challenge
                .is_some_and(|(sent, challenge)| challenge == tag && received > sent)
        {
            self.authentic = true;
        }
        if future || behind < u64::from(self.sync_ms()) {
            if notify {
                self.next = now + self.period(false)?;
                self.keeper = false;
                self.update = None;
            } else if self.update.is_none() {
                self.next = now + self.period(self.keeper)?;
            }
        }
        Ok((!notify && self.authentic).then_some(local.max(received)))
    }

    /// Report an actual secure routing transmission. Pending replies to stale devices survive.
    pub fn wrapper_sent(&mut self, now: Instant) -> io::Result<()> {
        self.time(now)?;
        if !self.authentic {
            return Err(invalid(SecureRoutingError::Clock));
        }
        if self.update.is_none() {
            self.next = now + self.period(self.keeper)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

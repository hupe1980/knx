//! Exchange KNX IP Secure routing datagrams using an externally synchronized timer or
//! [`GroupClock`]. Bind one receiver per commissioned sender. Preserve trusted time
//! across restarts and restoration of each previously used backbone key.

use crate::{
    GroupAddress, ProtectionEvidence, ip_secure_crypto,
    routing::{FlowControl, RoutingEvent, RoutingReceiver, SendOutcome, group_indication},
    telegram::{GroupService, GroupTelegram},
};
use std::{
    collections::BTreeSet,
    fmt,
    hash::{BuildHasher, RandomState},
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    time::Instant,
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

mod timer;
pub use timer::{GroupClock, TimerConfig, TimerOrigin, TimerStart, TimerStore};

const MAX_TIMER: u64 = (1 << 48) - 1;
const MAX_SEEN: usize = 512;
const MAX_PACKET: usize = 38 + 6 + GroupTelegram::MAX_ENCODED_LEN;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SecureRoutingError {
    Configuration,
    Clock,
    Binding,
    Authentication,
    Freshness,
    Replay,
    Capacity,
    InnerService,
}

impl fmt::Display for SecureRoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Configuration => {
                "Check the multicast address, interface index and latency tolerance."
            }
            Self::Clock => "Provide a trusted, nondecreasing 48-bit KNX group timer.",
            Self::Binding => "The datagram does not match the commissioned routing context.",
            Self::Authentication => "Secure routing authentication failed.",
            Self::Freshness => "The routing timestamp is outside the accepted window.",
            Self::Replay => "This sender timestamp and message tag were already delivered.",
            Self::Capacity => "The sender replay window is full.",
            Self::InnerService => "The authenticated payload is not a supported routing event.",
        })
    }
}
impl std::error::Error for SecureRoutingError {}

/// A routing event authenticated under the commissioned shared backbone key.
/// Any holder of that key can assert the sender serial. Group Data Secure is a separate layer.
pub struct VerifiedRouting(Zeroizing<Vec<u8>>);

impl fmt::Debug for VerifiedRouting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedRouting").finish_non_exhaustive()
    }
}

impl VerifiedRouting {
    pub fn event(&self) -> RoutingEvent<'_> {
        RoutingEvent::parse(&self.0).expect("validated immutable routing event")
    }

    pub const fn ip_protection(&self) -> ProtectionEvidence {
        ProtectionEvidence::AuthenticatedFresh
    }
}

/// Authenticated receive context for one KNX sender serial and multicast membership.
pub struct SecureRoutingReceiver {
    interface_index: u32,
    group: Ipv4Addr,
    serial: [u8; 6],
    key: Zeroizing<[u8; 16]>,
    latency_ms: u16,
    startup_timer: u64,
    last_timer: u64,
    seen: BTreeSet<(u64, u16)>,
}

impl SecureRoutingReceiver {
    /// Start from the current trusted group timer. Discard messages at or before this
    /// sample, including after restart or key rotation. A new key can start a new timer
    /// epoch. Restoring an old key requires trusted time above its prior uses.
    pub fn new(
        interface_index: u32,
        group: Ipv4Addr,
        serial: [u8; 6],
        backbone_key: [u8; 16],
        latency_ms: u16,
        trusted_timer_ms: u64,
    ) -> Result<Self, SecureRoutingError> {
        if interface_index == 0 || !group.is_multicast() || !(1..=8000).contains(&latency_ms) {
            return Err(SecureRoutingError::Configuration);
        }
        if trusted_timer_ms > MAX_TIMER {
            return Err(SecureRoutingError::Clock);
        }
        Ok(Self {
            interface_index,
            group,
            serial,
            key: Zeroizing::new(backbone_key),
            latency_ms,
            startup_timer: trusted_timer_ms,
            last_timer: trusted_timer_ms,
            seen: BTreeSet::new(),
        })
    }

    /// Supply the actual ingress, destination address and UDP port, and a fresh host timer sample.
    /// `None` rejects delivery while synchronization is unavailable. The receiver
    /// accepts reordered messages within the latency window and rejects future times.
    pub fn receive(
        &mut self,
        interface_index: u32,
        destination: SocketAddrV4,
        trusted_timer_ms: Option<u64>,
        packet: &[u8],
    ) -> Result<VerifiedRouting, SecureRoutingError> {
        if interface_index != self.interface_index
            || destination != SocketAddrV4::new(self.group, 3671)
        {
            return Err(SecureRoutingError::Binding);
        }
        let now = trusted_timer_ms
            .filter(|v| (self.last_timer..=MAX_TIMER).contains(v))
            .ok_or(SecureRoutingError::Clock)?;
        self.last_timer = now;
        if !(44..=MAX_PACKET).contains(&packet.len()) {
            return Err(SecureRoutingError::Authentication);
        }
        if packet[6..8] != [0, 0] || packet[14..20] != self.serial {
            return Err(SecureRoutingError::Binding);
        }
        let inner =
            ip_secure_crypto::open(&self.key, packet).ok_or(SecureRoutingError::Authentication)?;
        let mut bytes = [0; 8];
        bytes[2..].copy_from_slice(&packet[8..14]);
        let timestamp = u64::from_be_bytes(bytes);
        let oldest = now.saturating_sub(self.latency_ms.into());
        if timestamp <= self.startup_timer || !(oldest..=now).contains(&timestamp) {
            return Err(SecureRoutingError::Freshness);
        }
        RoutingEvent::parse(&inner).map_err(|_| SecureRoutingError::InnerService)?;
        let tag = u16::from_be_bytes([packet[20], packet[21]]);
        self.claim(oldest, timestamp, tag)?;
        Ok(VerifiedRouting(inner))
    }

    fn claim(&mut self, oldest: u64, timestamp: u64, tag: u16) -> Result<(), SecureRoutingError> {
        while self.seen.first().is_some_and(|&(time, _)| time < oldest) {
            self.seen.pop_first();
        }
        if self.seen.contains(&(timestamp, tag)) {
            return Err(SecureRoutingError::Replay);
        }
        if self.seen.len() >= MAX_SEEN {
            return Err(SecureRoutingError::Capacity);
        }
        self.seen.insert((timestamp, tag));
        Ok(())
    }
}

/// A paced application sender with an exclusive commissioned serial number.
/// Configure the socket's outgoing multicast interface before handing it to the sender.
/// Use a single sender for this serial across all processes and backbone key generations.
pub struct SecureRoutingSender {
    socket: UdpSocket,
    interface_index: u32,
    target: SocketAddrV4,
    address: u16,
    serial: [u8; 6],
    key: Zeroizing<[u8; 16]>,
    last_timer: u64,
    next_tag: u16,
    flow: FlowControl,
    jitter: RandomState,
}

impl SecureRoutingSender {
    /// Bind a commissioned individual address and dedicated serial to an unconnected
    /// IPv4 socket. Supply current rollback-resistant KNX group time after each restart.
    pub fn new(
        socket: UdpSocket,
        interface_index: u32,
        group: Ipv4Addr,
        address: u16,
        serial: [u8; 6],
        backbone_key: [u8; 16],
        trusted_timer_ms: u64,
    ) -> io::Result<Self> {
        let config = || {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                SecureRoutingError::Configuration,
            )
        };
        let SocketAddr::V4(local) = socket.local_addr()? else {
            return Err(config());
        };
        if interface_index == 0
            || !group.is_multicast()
            || socket.peer_addr().is_ok()
            || RoutingReceiver::new(*local.ip(), None).is_err()
        {
            return Err(config());
        }
        if trusted_timer_ms > MAX_TIMER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                SecureRoutingError::Clock,
            ));
        }
        Ok(Self {
            socket,
            interface_index,
            target: SocketAddrV4::new(group, 3671),
            address,
            serial,
            key: Zeroizing::new(backbone_key),
            last_timer: trusted_timer_ms,
            next_tag: 0,
            flow: FlowControl::new(Instant::now()),
            jitter: RandomState::new(),
        })
    }

    /// Authenticate a commissioned peer's datagram, applying verified busy intervals
    /// before returning its event. Use a distinct receiver for each permitted sender.
    pub fn receive(
        &mut self,
        peer: &mut SecureRoutingReceiver,
        ingress: u32,
        destination: SocketAddrV4,
        trusted_timer_ms: Option<u64>,
        packet: &[u8],
    ) -> Result<VerifiedRouting, SecureRoutingError> {
        if peer.interface_index != self.interface_index
            || peer.group != *self.target.ip()
            || !bool::from(peer.key.as_ref().ct_eq(self.key.as_ref()))
        {
            return Err(SecureRoutingError::Binding);
        }
        let verified = peer.receive(ingress, destination, trusted_timer_ms, packet)?;
        if let RoutingEvent::Busy { wait, .. } = verified.event() {
            let now = Instant::now();
            self.flow.busy(now, wait, self.jitter.hash_one(now) as u32);
        }
        Ok(verified)
    }

    /// Earliest monotonic send time after local rate limiting and verified peer busy reports.
    pub fn ready_at(&self) -> Instant {
        self.flow.ready_at()
    }

    /// Submit one encrypted group indication, or return a pacing deadline without sending.
    /// Sample the trusted group timer again on retry. Each send attempt consumes its
    /// timestamp even on socket error. The next attempt requires a newer timer tick.
    pub fn send(
        &mut self,
        trusted_timer_ms: Option<u64>,
        destination: GroupAddress,
        service: GroupService<'_>,
    ) -> io::Result<SendOutcome> {
        let mut inner = Zeroizing::new([0; 6 + GroupTelegram::MAX_ENCODED_LEN]);
        let inner = group_indication(self.address, destination, service, &mut inner)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let ready = self.ready_at();
        if Instant::now() < ready {
            return Ok(SendOutcome::RetryAt(ready));
        }
        let timer = trusted_timer_ms
            .filter(|v| *v > self.last_timer && *v <= MAX_TIMER)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, SecureRoutingError::Clock)
            })?;
        let sealed =
            ip_secure_crypto::seal_with_tag(&self.key, 0, timer, self.serial, self.next_tag, inner)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        SecureRoutingError::InnerService,
                    )
                })?;
        self.last_timer = timer;
        self.next_tag = self.next_tag.wrapping_add(1);
        self.socket.send_to(&sealed, self.target)?;
        self.flow.sent(Instant::now());
        Ok(SendOutcome::Sent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sender_consumes_each_nonce_even_on_socket_error_and_fences_restart() {
        use crate::telegram::GroupData;
        use std::time::Duration;
        let group = Ipv4Addr::new(239, 1, 2, 3);
        let dst: GroupAddress = "1/2/3".parse().unwrap();
        let sink = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        sink.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let SocketAddr::V4(target) = sink.local_addr().unwrap() else {
            unreachable!()
        };
        let create = |key, floor| {
            SecureRoutingSender::new(
                UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap(),
                1,
                group,
                0x1101,
                [1; 6],
                [key; 16],
                floor,
            )
            .unwrap()
        };
        let mut sender = create(0, 10);
        sender.target = target;
        let service = GroupService::Response(GroupData::Bytes(&[0x7d; 253]));
        for sample in [None, Some(10), Some(9), Some(1 << 48)] {
            assert_eq!(
                sender.send(sample, dst, service).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        assert_eq!(
            sender.send(Some(11), dst, service).unwrap(),
            SendOutcome::Sent
        );
        let mut buf = [0; MAX_PACKET];
        let len = sink.recv(&mut buf).unwrap();
        let mut rx = SecureRoutingReceiver::new(1, group, [1; 6], [0; 16], 100, 10).unwrap();
        let msg = rx
            .receive(1, SocketAddrV4::new(group, 3671), Some(11), &buf[..len])
            .unwrap();
        assert!(
            matches!(msg.event(),RoutingEvent::Indication(t) if t.source == 0x1101 && t.service == service)
        );
        assert!(matches!(
            sender.send(Some(12), dst, service).unwrap(),
            SendOutcome::RetryAt(_)
        ));
        sender.flow = FlowControl::new(Instant::now());
        // Exercise a kernel error without sending outside loopback.
        let wrong_family = UdpSocket::bind((std::net::Ipv6Addr::LOCALHOST, 0)).unwrap();
        let original = std::mem::replace(&mut sender.socket, wrong_family);
        assert!(sender.send(Some(12), dst, service).is_err());
        sender.socket = original;
        assert_eq!(
            sender.send(Some(12), dst, service).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            sender.send(Some(13), dst, service).unwrap(),
            SendOutcome::Sent
        );
        let len = sink.recv(&mut buf).unwrap();
        assert_eq!(&buf[20..22], &[0, 2]);
        rx.receive(1, SocketAddrV4::new(group, 3671), Some(13), &buf[..len])
            .unwrap();

        let mut rotated = create(1, 13);
        rotated.target = target;
        rotated.send(Some(14), dst, GroupService::Read).unwrap();
        let len = sink.recv(&mut buf).unwrap();
        assert_eq!(
            rx.receive(1, SocketAddrV4::new(group, 3671), Some(14), &buf[..len])
                .unwrap_err(),
            SecureRoutingError::Authentication
        );
        let mut restored = create(0, 14);
        restored.target = target;
        assert_eq!(
            restored.send(Some(14), dst, service).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        restored.send(Some(15), dst, service).unwrap();
        let len = sink.recv(&mut buf).unwrap();
        rx.receive(1, SocketAddrV4::new(group, 3671), Some(15), &buf[..len])
            .unwrap();
    }

    #[test]
    fn replay_window_retains_all_live_tags_on_capacity_and_accepts_reordering() {
        let mut r =
            SecureRoutingReceiver::new(1, Ipv4Addr::new(239, 1, 2, 3), [1; 6], [0; 16], 100, 0)
                .unwrap();
        for tag in 0..MAX_SEEN as u16 {
            r.claim(1, 3, tag).unwrap();
        }
        assert_eq!(r.claim(3, 4, 0), Err(SecureRoutingError::Capacity));
        assert_eq!(r.claim(3, 3, 1), Err(SecureRoutingError::Replay));
        r.claim(4, 5, 1).unwrap();
        r.claim(4, 4, 1).unwrap();
        r.claim(4, 5, 2).unwrap();
        assert_eq!(r.claim(4, 5, 1), Err(SecureRoutingError::Replay));
    }
}

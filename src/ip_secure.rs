//! Mutually authenticated KNXnet/IP Secure unicast sessions on a caller-owned TCP stream.
//!
//! Write each returned frame on the same connected stream and supply one complete
//! reply at a time. Drop all session state when the stream closes. Commission a
//! distinct KNX serial number for the client and load both 16-byte credential hashes.
use crate::{
    ip_core::IpFrame,
    ip_secure_crypto::{handshake_tag, open, seal},
};
use curve25519_dalek::montgomery::MontgomeryPoint;
use sha2::{Digest, Sha256};
use std::{
    fmt,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// Send a protected keepalive when the application has no outgoing traffic.
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
// Conservative client idle cutoff, measured from the last outgoing wrapper.
const IDLE_LIMIT: Duration = Duration::from_secs(30);
const MAX_SEQUENCE: u64 = (1 << 48) - 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Credentials,
    Entropy,
    Authentication,
    Frame,
    Replay,
    Expired,
    Closed,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Credentials => "Supply a commissioned unicast user and device credentials.",
            Self::Entropy => "System randomness is unavailable.",
            Self::Authentication => "Secure session authentication failed.",
            Self::Frame => "Supply one supported KNXnet/IP frame for this session.",
            Self::Replay => "The secure session sequence did not advance.",
            Self::Expired => "The secure session deadline elapsed.",
            Self::Closed => "Establish a fresh secure session.",
        })
    }
}
impl std::error::Error for Error {}

/// Single-use ephemeral client key and credentials for a pending session request.
pub struct Handshake {
    secret: Zeroizing<[u8; 32]>,
    request: [u8; 46],
    serial: [u8; 6],
    user: u8,
    device_key: Zeroizing<[u8; 16]>,
    user_key: Zeroizing<[u8; 16]>,
    deadline: Instant,
}
impl Handshake {
    /// Start a fresh exchange using OS randomness and commissioned password hashes.
    pub fn start(
        serial: [u8; 6],
        user: u8,
        device_key: [u8; 16],
        user_key: [u8; 16],
    ) -> Result<Self, Error> {
        if !(1..=127).contains(&user) {
            return Err(Error::Credentials);
        }
        let mut secret = Zeroizing::new([0; 32]);
        getrandom::fill(&mut *secret).map_err(|_| Error::Entropy)?;
        Ok(Self::from_secret(
            serial, user, device_key, user_key, secret,
        ))
    }
    fn from_secret(
        serial: [u8; 6],
        user: u8,
        device_key: [u8; 16],
        user_key: [u8; 16],
        secret: Zeroizing<[u8; 32]>,
    ) -> Self {
        let mut request = [0; 46];
        request[..8].copy_from_slice(&[6, 16, 9, 0x51, 0, 46, 8, 2]);
        request[14..].copy_from_slice(MontgomeryPoint::mul_base_clamped(*secret).as_bytes());
        Self {
            secret,
            request,
            serial,
            user,
            device_key: Zeroizing::new(device_key),
            user_key: Zeroizing::new(user_key),
            deadline: Instant::now() + HANDSHAKE_TIMEOUT,
        }
    }
    /// Send this SESSION_REQUEST as the first frame on the selected TCP stream.
    pub fn request(&self) -> &[u8; 46] {
        &self.request
    }

    /// Verify the device proof and prepare the single protected user authentication request.
    pub fn authenticate(self, response: &[u8]) -> Result<(AwaitingStatus, Vec<u8>), Error> {
        if Instant::now() >= self.deadline {
            return Err(Error::Expired);
        }
        let frame = IpFrame::parse(response).map_err(|_| Error::Frame)?;
        if frame.service != 0x0952 || response.len() != 56 {
            return Err(Error::Frame);
        }
        let id = u16::from_be_bytes([response[6], response[7]]);
        if id == 0 {
            return Err(Error::Authentication);
        }
        let peer: [u8; 32] = response[8..40].try_into().expect("fixed response length");
        let mut associated = [0; 40];
        associated[..8].copy_from_slice(&response[..8]);
        for ((dst, a), b) in associated[8..]
            .iter_mut()
            .zip(&peer)
            .zip(&self.request[14..])
        {
            *dst = a ^ b;
        }
        if !bool::from(handshake_tag(&self.device_key, &associated).ct_eq(&response[40..])) {
            return Err(Error::Authentication);
        }
        let shared = Zeroizing::new(MontgomeryPoint(peer).mul_clamped(*self.secret).to_bytes());
        if bool::from(shared.ct_eq(&[0; 32])) {
            return Err(Error::Authentication);
        }
        let mut digest = Sha256::digest(&shared[..]);
        let mut key = Zeroizing::new([0; 16]);
        key.copy_from_slice(&digest[..16]);
        digest.as_mut_slice().zeroize();
        let mut auth = Zeroizing::new([0; 24]);
        auth[..8].copy_from_slice(&[6, 16, 9, 0x53, 0, 24, 0, self.user]);
        associated[..8].copy_from_slice(&auth[..8]);
        auth[8..].copy_from_slice(&handshake_tag(&self.user_key, &associated));
        let wrapped = seal(&key, id, 0, self.serial, &auth[..]).ok_or(Error::Frame)?;
        let session = Session {
            key,
            id,
            serial: self.serial,
            peer_serial: None,
            next: 1,
            received: None,
            deadline: Instant::now() + HANDSHAKE_TIMEOUT,
            closed: false,
        };
        Ok((AwaitingStatus(session), wrapped))
    }
}

/// Awaiting the server's protected authentication result. Application traffic starts after finish.
pub struct AwaitingStatus(Session);
impl AwaitingStatus {
    pub fn finish(mut self, packet: &[u8]) -> Result<Session, Error> {
        let plaintext = self.0.unwrap(packet)?;
        if plaintext.as_slice() != [6, 16, 9, 0x54, 0, 8, 0, 0] {
            return Err(Error::Authentication);
        }
        self.0.deadline = Instant::now() + IDLE_LIMIT;
        Ok(self.0)
    }
}

/// A KNXnet/IP frame verified in this specific TCP session.
pub struct VerifiedFrame(Zeroizing<Vec<u8>>);
impl VerifiedFrame {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub const fn ip_protection(&self) -> crate::ProtectionEvidence {
        crate::ProtectionEvidence::AuthenticatedFresh
    }
}
pub enum Received {
    Frame(VerifiedFrame),
    Keepalive,
    Closed(u8),
}

/// Each session owns its ephemeral key, directional counters and peer serial binding.
pub struct Session {
    key: Zeroizing<[u8; 16]>,
    id: u16,
    serial: [u8; 6],
    peer_serial: Option<[u8; 6]>,
    next: u64,
    received: Option<u64>,
    deadline: Instant,
    closed: bool,
}
fn supported(service: u16) -> bool {
    matches!(service, 0x0203..=0x020a | 0x0420..=0x0421)
}
impl Session {
    fn live(&mut self) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if Instant::now() >= self.deadline {
            self.end();
            return Err(Error::Expired);
        }
        Ok(())
    }
    fn end(&mut self) {
        self.key.zeroize();
        self.closed = true;
    }
    fn wrap(&mut self, plain: &[u8]) -> Result<Vec<u8>, Error> {
        self.live()?;
        if self.next > MAX_SEQUENCE {
            self.end();
            return Err(Error::Closed);
        }
        let packet = seal(&self.key, self.id, self.next, self.serial, plain).ok_or(Error::Frame)?;
        self.next += 1;
        self.deadline = Instant::now() + IDLE_LIMIT;
        Ok(packet)
    }
    fn unwrap(&mut self, packet: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
        self.live()?;
        if packet.len() < 44 || packet[6..8] != self.id.to_be_bytes() || packet[20..22] != [0, 0] {
            return Err(Error::Frame);
        }
        let serial: [u8; 6] = packet[14..20].try_into().expect("length checked");
        if serial == self.serial || self.peer_serial.is_some_and(|s| s != serial) {
            return Err(Error::Authentication);
        }
        let mut seq = [0; 8];
        seq[2..].copy_from_slice(&packet[8..14]);
        let sequence = u64::from_be_bytes(seq);
        if self.received.is_some_and(|seen| sequence <= seen) {
            return Err(Error::Replay);
        }
        let plain = open(&self.key, packet).ok_or(Error::Authentication)?;
        IpFrame::parse(&plain).map_err(|_| Error::Frame)?;
        self.received = Some(sequence);
        self.peer_serial = Some(serial);
        Ok(plain)
    }
    /// Protect one complete core description, connection management or tunnelling frame.
    pub fn send(&mut self, plain: &[u8]) -> Result<Vec<u8>, Error> {
        let frame = IpFrame::parse(plain).map_err(|_| Error::Frame)?;
        if !supported(frame.service) {
            return Err(Error::Frame);
        }
        self.wrap(plain)
    }
    /// Emit a protected keepalive before the peer's inactivity deadline.
    pub fn keepalive(&mut self) -> Result<Vec<u8>, Error> {
        self.wrap(&[6, 16, 9, 0x54, 0, 8, 4, 0])
    }
    /// Emit a protected close and discard the session key.
    pub fn close(mut self) -> Result<Vec<u8>, Error> {
        self.wrap(&[6, 16, 9, 0x54, 0, 8, 5, 0])
    }
    /// Check authentication and sequence before releasing plaintext to the application.
    pub fn receive(&mut self, packet: &[u8]) -> Result<Received, Error> {
        let plain = self.unwrap(packet)?;
        let frame = IpFrame::parse(&plain).map_err(|_| Error::Frame)?;
        if supported(frame.service) {
            return Ok(Received::Frame(VerifiedFrame(plain)));
        }
        match (frame.service, frame.body) {
            (0x0954, [4, 0]) => Ok(Received::Keepalive),
            (0x0954, [code @ (1..=3 | 5), 0]) => {
                self.end();
                Ok(Received::Closed(*code))
            }
            _ => Err(Error::Frame),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent fixtures using XKNX 4.1.1 SecureSession.handshake/encrypt_frame.
    const RESPONSE: &str = "061009520038002a5dfedd3b6bd47f6fa28ee15d969d5bb0ea53774d488bdaf9df1c6e0124b3ef22dabdf9fee8bc2ffa98749901c03a00df";
    const AUTH: &str = "06100950003e002a0000000000000000786b6e780000c29b6a8cc82a233c76dcb5c2e0586d91c22521570f3b7be73aa87b8fba2f89f99d5cd9a3ad84e30e";
    const STATUS: &str = "06100950002e002a000000000000000000000123000040cfaff7bb3533991aec5a45714a1784816340d688f642b9";
    const KEEP: &str = "06100950002e002a0000000000010000000001230000d6c2288d0f01c93466e618604fc8cff32a5bf97b874c23e7";
    const CLOSE: &str = "06100950002e002a00000000000200000000012300002d13ca6884c2f1103a2133f685cc4331e46ca5ae959b9de0";

    fn hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    fn start() -> Handshake {
        Handshake::from_secret(
            [0, 0, 0x78, 0x6b, 0x6e, 0x78],
            2,
            [0x12; 16],
            [0x34; 16],
            Zeroizing::new([7; 32]),
        )
    }
    fn session() -> Session {
        start()
            .authenticate(&hex(RESPONSE))
            .unwrap()
            .0
            .finish(&hex(STATUS))
            .unwrap()
    }

    #[test]
    fn an159_v06_encrypted_device_proof() {
        // AN159v06 SessionResponse example, reproduced with the XKNX CBC and CTR primitives.
        let key: [u8; 16] = hex("e158e4012047bd6cc41aafbc5c04c1fc").try_into().unwrap();
        let fields =
            hex("0610095200380001b752be246459260f6b0c4801fbd5a67599f83b4057b3ef1e79e469ac17234e15");
        assert_eq!(
            handshake_tag(&key, &fields).as_slice(),
            hex("a922505aaa436163570bd5494c2df2a3")
        );
    }
    #[test]
    fn independent_handshake_and_directional_sequences() {
        let h = start();
        assert_eq!(
            h.request().as_slice(),
            hex(
                "06100951002e080200000000000013be4feaeaf204c7fd3358fc9c00721881d174278128227ec674f37f7fe97b6d"
            )
        );
        let (pending, auth) = h.authenticate(&hex(RESPONSE)).unwrap();
        assert_eq!(auth, hex(AUTH));
        let mut session = pending.finish(&hex(STATUS)).unwrap();
        let mut tampered = hex(KEEP);
        tampered[13] = 42;
        assert!(matches!(
            session.receive(&tampered),
            Err(Error::Authentication)
        ));
        assert!(matches!(
            session.receive(&hex(KEEP)),
            Ok(Received::Keepalive)
        ));
        assert!(matches!(session.receive(&hex(KEEP)), Err(Error::Replay)));
        assert!(matches!(
            session.receive(&hex(CLOSE)),
            Ok(Received::Closed(5))
        ));
        assert!(matches!(session.keepalive(), Err(Error::Closed)));
    }
    #[test]
    fn mutual_proofs_deadlines_and_key_exhaustion_fail_closed() {
        let mut expired = start();
        expired.deadline = Instant::now();
        assert!(matches!(
            expired.authenticate(&hex(RESPONSE)),
            Err(Error::Expired)
        ));
        for i in [6, 8, 39, 40, 55] {
            let mut damaged = hex(RESPONSE);
            damaged[i] ^= 1;
            assert!(matches!(
                start().authenticate(&damaged),
                Err(Error::Authentication)
            ));
        }
        let mut low_order = hex(RESPONSE);
        low_order[8..40].fill(0);
        let h = start();
        let mut ad = [0; 40];
        ad[..8].copy_from_slice(&low_order[..8]);
        ad[8..].copy_from_slice(&h.request()[14..]);
        low_order[40..].copy_from_slice(&handshake_tag(&[0x12; 16], &ad));
        assert!(matches!(
            h.authenticate(&low_order),
            Err(Error::Authentication)
        ));
        let (pending, _) = start().authenticate(&hex(RESPONSE)).unwrap();
        assert!(matches!(
            pending.finish(&hex(KEEP)),
            Err(Error::Authentication)
        ));
        let (mut pending, _) = start().authenticate(&hex(RESPONSE)).unwrap();
        pending.0.deadline = Instant::now();
        assert!(matches!(pending.finish(&hex(STATUS)), Err(Error::Expired)));
        let mut s = session();
        s.next = MAX_SEQUENCE;
        assert_eq!(&s.keepalive().unwrap()[8..14], &[255; 6]);
        assert_eq!(s.keepalive(), Err(Error::Closed));
        let mut s = session();
        s.deadline = Instant::now();
        assert!(matches!(s.receive(&hex(KEEP)), Err(Error::Expired)));
        assert!(matches!(s.receive(&hex(KEEP)), Err(Error::Closed)));
    }
    #[test]
    fn record_limits_session_binding_and_plain_rejection() {
        let mut s = session();
        for i in [6, 7, 14, 20, 21, 22, 45] {
            let mut damaged = hex(KEEP);
            damaged[i] ^= 1;
            assert!(s.receive(&damaged).is_err());
        }
        assert!(s.receive(&[6, 16, 4, 0x20, 0, 6]).is_err());
        assert_eq!(s.send(&[6, 16, 9, 0x50, 0, 6]), Err(Error::Frame));
        let plain = |size: usize| {
            let mut frame = vec![0; size];
            frame[..4].copy_from_slice(&[6, 16, 4, 0x20]);
            frame[4..6].copy_from_slice(&(size as u16).to_be_bytes());
            frame
        };
        assert_eq!(s.send(&plain(4081)), Err(Error::Frame));
        let record = s.send(&plain(4080)).unwrap();
        assert_eq!(record.len(), 4118);
        assert_eq!(&record[8..14], &[0, 0, 0, 0, 0, 1]);
        assert!(s.receive(&record).is_err());
        assert!(matches!(s.receive(&hex(KEEP)), Ok(Received::Keepalive)));
    }
}

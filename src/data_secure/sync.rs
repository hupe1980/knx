//! Normal-mode sequence synchronization with a commissioned, individually addressed peer.
//! The peer uses a point-to-point key and both directions use authenticated encryption.

use super::{MAX_SEQUENCE, OutgoingSequenceStore, SecureRequest, auth_tag, counter_block, crypt};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncError {
    Binding,
    Unsupported,
    Authentication,
    Expired,
    Completed,
    RateLimited,
    StateUnavailable,
    SequenceExhausted,
    RandomUnavailable,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Binding => "Check the local and peer individual addresses.",
            Self::Unsupported => {
                "Expected a normal-mode connectionless synchronization indication."
            }
            Self::Authentication => "Sequence synchronization authentication failed.",
            Self::Expired => "The synchronization exchange timed out.",
            Self::Completed => "The synchronization response was already applied.",
            Self::RateLimited => "Wait at least one second between synchronization responses.",
            Self::StateUnavailable => "Restore trusted normal-mode sequence state.",
            Self::SequenceExhausted => "The normal-mode sequence is exhausted.",
            Self::RandomUnavailable => "Secure randomness is unavailable.",
        })
    }
}
impl std::error::Error for SyncError {}

/// The *next* local sending sequence and the next sequence expected from the peer.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NextSequences {
    pub local: u64,
    pub peer: u64,
}
impl NextSequences {
    fn valid(self) -> bool {
        (1..=MAX_SEQUENCE).contains(&self.local) && (1..=MAX_SEQUENCE).contains(&self.peer)
    }
}

/// Provenance of the authenticated lower bounds supplied to the durable store.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SyncUpdate {
    /// The incoming request may have been recorded earlier. Require an existing,
    /// trusted incoming baseline before replying.
    PeerRequest,
    /// The peer answered this process's outstanding fresh challenge within its deadline.
    MatchedResponse,
}

/// Reconcile the same durable normal-mode counters used by GroupSender and GroupReceiver.
/// In one transaction, raise each next sequence to at least its supplied floor and
/// return both committed values. Preserve every higher reservation or accepted frame,
/// including concurrent group updates. A high-water mark stores `next - 1`.
/// Return an error on uncertain or missing local state, exhaustion or commit failure.
/// Only a MatchedResponse may establish a missing incoming baseline. A PeerRequest
/// can be replayed and must fail when that baseline is unavailable.
pub trait SyncStore {
    type Error;
    fn reconcile(
        &mut self,
        local: u16,
        peer: u16,
        floor: NextSequences,
        update: SyncUpdate,
    ) -> Result<NextSequences, Self::Error>;
}

/// Commissioned normal-mode point-to-point key belonging to this local/peer pair.
pub struct SyncPeer {
    local: u16,
    peer: u16,
    key: Zeroizing<[u8; 16]>,
}
impl SyncPeer {
    pub fn new(local: u16, peer: u16, point_to_point_key: [u8; 16]) -> Result<Self, SyncError> {
        if local == peer {
            return Err(SyncError::Binding);
        }
        Ok(Self {
            local,
            peer,
            key: Zeroizing::new(point_to_point_key),
        })
    }

    /// Replace the commissioned point-to-point key. A pending exchange borrows
    /// this binding, so it must finish or be discarded before the replacement.
    /// Keep the same durable normal-mode sequence store across this operation.
    ///
    /// ```compile_fail
    /// use knx::data_secure::{OutgoingSequenceStore, sync::{SyncPeer, SyncStore}};
    /// use std::time::Instant;
    /// fn rotate(binding: &mut SyncPeer, store: &mut (impl OutgoingSequenceStore + SyncStore),
    ///           new_key: [u8;16], delayed: &[u8]) {
    ///     let (_, mut pending) = binding.request(Instant::now(), store).unwrap();
    ///     binding.replace_key(new_key);
    ///     pending.complete(delayed, Instant::now(), store).unwrap();
    /// }
    /// ```
    pub fn replace_key(&mut self, point_to_point_key: [u8; 16]) {
        self.key = Zeroizing::new(point_to_point_key);
    }

    /// Begin a six-second exchange and durably consume a unique request sequence.
    pub fn request(
        &self,
        now: Instant,
        store: &mut impl OutgoingSequenceStore,
    ) -> Result<(SecureRequest, PendingSync<'_>), SyncError> {
        let challenge = random6()?;
        let number = store
            .reserve(self.local)
            .map_err(|_| SyncError::StateUnavailable)?
            .ok_or(SyncError::SequenceExhausted)?;
        if !(1..=MAX_SEQUENCE).contains(&number) {
            return Err(SyncError::StateUnavailable);
        }
        let nonce = six(number);
        let request = self.seal(&nonce, &nonce, &REQUEST_AD, challenge);
        Ok((
            request,
            PendingSync {
                binding: self,
                challenge,
                started: now,
                finished: false,
            },
        ))
    }

    fn seal<const N: usize>(
        &self,
        nonce: &[u8; 6],
        wire_nonce: &[u8; 6],
        ad: &[u8],
        mut payload: [u8; N],
    ) -> SecureRequest {
        let ctr = counter_block(nonce, self.local, self.peer);
        let mut tag = auth_tag(&self.key, &ctr, 0, ad, &payload);
        crypt(&self.key, &ctr, &mut tag, &mut payload);
        let mut frame = Vec::with_capacity(34);
        // Extended cEMI, system priority, individual destination, six hops.
        frame.extend([0x11, 0, 0x30, 0x60]);
        frame.extend(self.local.to_be_bytes());
        frame.extend(self.peer.to_be_bytes());
        frame.extend([24, 3, 0xf1, ad[0]]);
        frame.extend(wire_nonce);
        frame.extend(&ad[1..]);
        frame.extend(payload);
        frame.extend(tag);
        SecureRequest { frame }
    }

    fn open<const N: usize>(
        &self,
        frame: &[u8],
        ad: &[u8],
        mask: &[u8; 6],
    ) -> Result<([u8; 6], Zeroizing<[u8; N]>), SyncError> {
        let [0x29, info_len, rest @ ..] = frame else {
            return Err(SyncError::Unsupported);
        };
        let (info, body) = rest
            .split_at_checked(usize::from(*info_len))
            .ok_or(SyncError::Unsupported)?;
        let [
            control,
            routing,
            sh,
            sl,
            dh,
            dl,
            24,
            3,
            0xf1,
            scf,
            tail @ ..,
        ] = body
        else {
            return Err(SyncError::Unsupported);
        };
        if !matches!(info, [] | [4, 2, _, _])
            || control & 0xd1 != 0x10
            || routing & 0x8f != 0
            || *scf != ad[0]
            || tail.len() != 22
        {
            return Err(SyncError::Unsupported);
        }
        if u16::from_be_bytes([*sh, *sl]) != self.peer
            || u16::from_be_bytes([*dh, *dl]) != self.local
        {
            return Err(SyncError::Binding);
        }
        let nonce: [u8; 6] = std::array::from_fn(|i| tail[i] ^ mask[i]);
        let extra = ad.len() - 1;
        if tail[6..6 + extra] != ad[1..] {
            return Err(SyncError::Unsupported);
        }
        let ctr = counter_block(&nonce, self.peer, self.local);
        let mut payload = Zeroizing::new(
            <[u8; N]>::try_from(&tail[6 + extra..18]).map_err(|_| SyncError::Unsupported)?,
        );
        let mut tag = <[u8; 4]>::try_from(&tail[18..]).expect("four-byte tag");
        crypt(&self.key, &ctr, &mut tag, &mut payload[..]);
        if !bool::from(tag.ct_eq(&auth_tag(&self.key, &ctr, 0, ad, &payload[..]))) {
            return Err(SyncError::Authentication);
        }
        Ok((nonce, payload))
    }

    fn commit(
        &self,
        store: &mut impl SyncStore,
        floor: NextSequences,
        update: SyncUpdate,
    ) -> Result<NextSequences, SyncError> {
        if !floor.valid() {
            return Err(SyncError::Unsupported);
        }
        let applied = store
            .reconcile(self.local, self.peer, floor, update)
            .map_err(|_| SyncError::StateUnavailable)?;
        if !applied.valid() || applied.local < floor.local || applied.peer < floor.peer {
            return Err(SyncError::StateUnavailable);
        }
        Ok(applied)
    }
}

const REQUEST_AD: [u8; 7] = [0x12, 0, 0, 0, 0, 0, 0];
const RESPONSE_AD: [u8; 1] = [0x13];
fn random6() -> Result<[u8; 6], SyncError> {
    let mut value = [0; 6];
    getrandom::fill(&mut value).map_err(|_| SyncError::RandomUnavailable)?;
    Ok(value)
}
fn six(n: u64) -> [u8; 6] {
    n.to_be_bytes()[2..].try_into().expect("six bytes")
}
fn number(bytes: &[u8]) -> u64 {
    let mut n = [0; 8];
    n[2..].copy_from_slice(bytes);
    u64::from_be_bytes(n)
}

/// Outstanding challenge bound to one local/peer/key tuple.
pub struct PendingSync<'a> {
    binding: &'a SyncPeer,
    challenge: [u8; 6],
    started: Instant,
    finished: bool,
}
impl PendingSync<'_> {
    /// Accept one authenticated response before the deadline. Binding/tag failures
    /// leave the challenge live. A failed commit consumes nothing unless the host committed it.
    pub fn complete(
        &mut self,
        frame: &[u8],
        now: Instant,
        store: &mut impl SyncStore,
    ) -> Result<(), SyncError> {
        if self.finished {
            return Err(SyncError::Completed);
        }
        if now
            .checked_duration_since(self.started)
            .is_none_or(|age| age >= Duration::from_secs(6))
        {
            self.finished = true;
            return Err(SyncError::Expired);
        }
        let (_, data) = self
            .binding
            .open::<12>(frame, &RESPONSE_AD, &self.challenge)?;
        self.binding.commit(
            store,
            NextSequences {
                local: number(&data[6..]),
                peer: number(&data[..6]),
            },
            SyncUpdate::MatchedResponse,
        )?;
        self.finished = true;
        Ok(())
    }
}

/// One responder per local individual address. Share it across all commissioned peers.
pub struct SyncResponder {
    local: u16,
    last: Option<Instant>,
}
impl SyncResponder {
    pub const fn new(local: u16) -> Self {
        Self { local, last: None }
    }

    /// Authenticate an individual sync request and reply with the durable next
    /// sequence numbers. Repeated requests can never lower either counter.
    pub fn respond(
        &mut self,
        binding: &SyncPeer,
        frame: &[u8],
        now: Instant,
        store: &mut impl SyncStore,
    ) -> Result<SecureRequest, SyncError> {
        if self.local != binding.local {
            return Err(SyncError::Binding);
        }
        if self.last.is_some_and(|last| {
            now.checked_duration_since(last)
                .is_none_or(|age| age < Duration::from_secs(1))
        }) {
            return Err(SyncError::RateLimited);
        }
        let (seq, challenge) = binding.open::<6>(frame, &REQUEST_AD, &[0; 6])?;
        let nonce = random6()?;
        let state = binding.commit(
            store,
            NextSequences {
                local: 1,
                peer: number(&seq),
            },
            SyncUpdate::PeerRequest,
        )?;
        let masked = std::array::from_fn(|i| nonce[i] ^ challenge[i]);
        let mut payload = [0; 12];
        payload[..6].copy_from_slice(&six(state.local));
        payload[6..].copy_from_slice(&six(state.peer));
        self.last = Some(now);
        Ok(binding.seal(&nonce, &masked, &RESPONSE_AD, payload))
    }
}

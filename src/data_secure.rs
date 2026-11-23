//! Send and receive KNX Data Secure authenticated-and-encrypted group traffic.
//!
//! Bind each group key to its commissioned group and individual address. The host
//! atomically commits received high-water marks and reserves outgoing sequences in
//! separate durable records. Share each sender’s normal sequence across group keys.
pub mod sync;

use crate::{
    GroupAddress,
    telegram::{GroupService, GroupTelegram, decode_service},
};
use aes::{
    Aes128,
    cipher::{BlockEncryptMut, KeyIvInit, StreamCipher},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Outcome of authenticating and accepting a group indication.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReceiveError {
    Unsupported,
    Binding,
    Authentication,
    Replay,
    StateUnavailable,
}

impl std::fmt::Display for ReceiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "Use a supported Data Secure group indication.",
            Self::Binding => "Check the commissioned sender and group.",
            Self::Authentication => "Group authentication failed.",
            Self::Replay => "The sender sequence did not advance.",
            Self::StateUnavailable => "Restore trusted sender sequence state before receiving.",
        })
    }
}
impl std::error::Error for ReceiveError {}

/// Compare against the persisted normal-mode sequence for this installation and sender.
/// Return true only after atomically committing a strictly higher sequence, false
/// for a replay, or an error if the baseline is missing or cannot be committed.
/// Preserve the high-water mark across restarts, group changes and key rotation.
pub trait SequenceStore {
    type Error;
    fn advance(&mut self, sender: u16, sequence: u64) -> Result<bool, Self::Error>;
}

/// A verified group APDU whose sender sequence has already been committed.
pub struct VerifiedGroup {
    apdu: Zeroizing<Vec<u8>>,
    pub sequence: u64,
}

impl VerifiedGroup {
    /// Group-layer evidence for checking this delivery with [`crate::DeliveryPolicy`].
    pub const fn protection(&self) -> crate::ProtectionEvidence {
        crate::ProtectionEvidence::AuthenticatedFresh
    }

    /// Decode the authenticated application service.
    pub fn service(&self) -> GroupService<'_> {
        decode_service(self.apdu[0], self.apdu[1], &self.apdu[2..])
            .expect("validated at the receive boundary")
    }
}

/// Receive binding for one authorized individual address and group address.
pub struct GroupReceiver {
    sender: u16,
    group: GroupAddress,
    key: Zeroizing<[u8; 16]>,
}

impl GroupReceiver {
    pub fn new(sender: u16, group: GroupAddress, key: [u8; 16]) -> Self {
        Self {
            sender,
            group,
            key: Zeroizing::new(key),
        }
    }

    /// Open one cEMI L_Data.ind with no additional information or a relative timestamp.
    /// Only connectionless S-A-Data with normal group-key access and format zero is accepted.
    pub fn receive(
        &self,
        frame: &[u8],
        store: &mut impl SequenceStore,
    ) -> Result<VerifiedGroup, ReceiveError> {
        let [0x29, info_len, rest @ ..] = frame else {
            return Err(ReceiveError::Unsupported);
        };
        let (info, body) = rest
            .split_at_checked(usize::from(*info_len))
            .ok_or(ReceiveError::Unsupported)?;
        if !matches!(info, [] | [4, 2, _, _]) {
            return Err(ReceiveError::Unsupported);
        }
        let [
            control,
            routing,
            sh,
            sl,
            gh,
            gl,
            length,
            3,
            0xf1,
            0x10,
            protected @ ..,
        ] = body
        else {
            return Err(ReceiveError::Unsupported);
        };
        if control & 0x51 != 0x10
            || routing & 0x8f != 0x80
            || body.len() != 8 + usize::from(*length)
            || (control & 0x80 != 0 && *length > 15)
            || *length == 255
            || protected.len() < 12
        {
            return Err(ReceiveError::Unsupported);
        }
        if u16::from_be_bytes([*sh, *sl]) != self.sender
            || u16::from_be_bytes([*gh, *gl]) != u16::from(self.group)
        {
            return Err(ReceiveError::Binding);
        }
        let (sequence, data) = protected.split_at(6);
        let (encrypted, tag) = data.split_at(data.len() - 4);
        let counter = counter_block(sequence, self.sender, self.group.into());
        let mut plain_tag = <[u8; 4]>::try_from(tag).expect("four-byte tag");
        let mut apdu = Zeroizing::new(encrypted.to_vec());
        crypt(&self.key, &counter, &mut plain_tag, &mut apdu);
        if !bool::from(plain_tag.ct_eq(&auth_tag(&self.key, &counter, 0x80, &[0x10], &apdu))) {
            return Err(ReceiveError::Authentication);
        }
        decode_service(apdu[0], apdu[1], &apdu[2..]).map_err(|_| ReceiveError::Unsupported)?;
        let mut number = [0u8; 8];
        number[2..].copy_from_slice(sequence);
        let sequence = u64::from_be_bytes(number);
        if sequence == 0
            || !store
                .advance(self.sender, sequence)
                .map_err(|_| ReceiveError::StateUnavailable)?
        {
            return Err(ReceiveError::Replay);
        }
        Ok(VerifiedGroup { apdu, sequence })
    }
}

/// Highest normal-mode sequence number representable on the bus.
pub const MAX_SEQUENCE: u64 = (1 << 48) - 1;

/// Atomically reserve the next unused normal-mode sequence for this installation and sender.
/// Return a positive 48-bit value only after its high-water mark is durably committed.
/// Return `Ok(None)` at exhaustion and an error on missing or uncertain state.
/// Share reservations across processes, groups and key rotations. Never reclaim a
/// reservation after a crash, dropped request or failed transport delivery.
pub trait OutgoingSequenceStore {
    type Error;
    fn reserve(&mut self, sender: u16) -> Result<Option<u64>, Self::Error>;
}

/// Failure to produce an authenticated request.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SendError {
    InvalidPayload,
    StateUnavailable,
    SequenceExhausted,
}
impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPayload => "Use six inline bits or 1 to 240 external application octets.",
            Self::StateUnavailable => "Restore trusted outgoing sequence state before sending.",
            Self::SequenceExhausted => "The outgoing sender sequence is exhausted.",
        })
    }
}
impl std::error::Error for SendError {}

/// A sealed cEMI L_Data.req bound to its commissioned source and destination.
/// This can carry a group service or an individual sequence synchronization service.
pub struct SecureRequest {
    frame: Vec<u8>,
}
impl SecureRequest {
    /// Individual address included in the authentication calculation.
    pub fn source(&self) -> u16 {
        u16::from_be_bytes([self.frame[4], self.frame[5]])
    }
    /// Complete authenticated cEMI request for the link transport.
    pub fn as_bytes(&self) -> &[u8] {
        &self.frame
    }
}

/// Send binding for one commissioned individual address, group address and normal key.
///
/// ```
/// use knx::{GroupAddress, data_secure::{GroupSender, OutgoingSequenceStore, SecureRequest, SendError},
///           telegram::{GroupData, GroupService}};
///
/// fn seal_switch_on(source: u16, group: GroupAddress, key: [u8; 16],
///                   reservations: &mut impl OutgoingSequenceStore) -> Result<SecureRequest, SendError> {
///     GroupSender::new(source, group, key)
///         .request(GroupService::Write(GroupData::Inline(1)), reservations)
/// }
/// ```
pub struct GroupSender {
    sender: u16,
    group: GroupAddress,
    key: Zeroizing<[u8; 16]>,
}

impl GroupSender {
    pub fn new(sender: u16, group: GroupAddress, key: [u8; 16]) -> Self {
        Self {
            sender,
            group,
            key: Zeroizing::new(key),
        }
    }

    /// Seal a read, response or write after a successful durable sequence reservation.
    /// A transport retry must reuse the original sealed bytes.
    pub fn request(
        &self,
        service: GroupService<'_>,
        store: &mut impl OutgoingSequenceStore,
    ) -> Result<SecureRequest, SendError> {
        // Reuse the ordinary group codec to validate the inner APDU before consuming
        // a reservation. A request without additional information has a nine-byte prefix.
        let mut plain = Zeroizing::new([0; 9 + 242]);
        let len = GroupTelegram::request(self.sender, self.group, service)
            .encode(&mut plain[..])
            .map_err(|_| SendError::InvalidPayload)?;
        let apdu = &mut plain[9..len];
        let number = store
            .reserve(self.sender)
            .map_err(|_| SendError::StateUnavailable)?
            .ok_or(SendError::SequenceExhausted)?;
        if !(1..=MAX_SEQUENCE).contains(&number) {
            return Err(SendError::StateUnavailable);
        }
        let sequence = number.to_be_bytes();
        let counter = counter_block(&sequence[2..], self.sender, self.group.into());
        let mut tag = auth_tag(&self.key, &counter, 0x80, &[0x10], apdu);
        crypt(&self.key, &counter, &mut tag, apdu);
        let length = 12 + apdu.len() as u8;
        let mut frame = Vec::with_capacity(10 + usize::from(length));
        frame.extend([0x11, 0, if length <= 15 { 0xbc } else { 0x3c }, 0xe0]);
        frame.extend(self.sender.to_be_bytes());
        frame.extend(u16::from(self.group).to_be_bytes());
        frame.extend([length, 3, 0xf1, 0x10]);
        frame.extend_from_slice(&sequence[2..]);
        frame.extend_from_slice(apdu);
        frame.extend(tag);
        Ok(SecureRequest { frame })
    }
}

fn counter_block(sequence: &[u8], source: u16, destination: u16) -> [u8; 16] {
    let mut counter = [0; 16];
    counter[..6].copy_from_slice(sequence);
    counter[6..8].copy_from_slice(&source.to_be_bytes());
    counter[8..10].copy_from_slice(&destination.to_be_bytes());
    counter[14] = 1;
    counter
}

// KNX 03/03/07 §5.1.3.2 places the tag first in the stream and last on the wire.
fn crypt(key: &[u8; 16], counter: &[u8; 16], tag: &mut [u8; 4], apdu: &mut [u8]) {
    let mut stream = ctr::Ctr128BE::<Aes128>::new(key.into(), counter.into());
    stream.apply_keystream(tag);
    stream.apply_keystream(apdu);
}

fn auth_tag(
    key: &[u8; 16],
    counter: &[u8; 16],
    address_type: u8,
    associated: &[u8],
    apdu: &[u8],
) -> [u8; 4] {
    // B0, length-prefixed SCF, APDU and zero padding.
    let mut input = Zeroizing::new(Vec::with_capacity(
        (18 + associated.len() + apdu.len()).next_multiple_of(16),
    ));
    input.extend_from_slice(counter);
    input[10..].copy_from_slice(&[0, address_type, 3, 0xf1, 0, apdu.len() as u8]);
    input.extend((associated.len() as u16).to_be_bytes());
    input.extend_from_slice(associated);
    input.extend_from_slice(apdu);
    let padded = input.len().next_multiple_of(16);
    input.resize(padded, 0);
    let mut cipher = cbc::Encryptor::<Aes128>::new(key.into(), (&[0u8; 16]).into());
    for block in input.as_chunks_mut::<16>().0 {
        cipher.encrypt_block_mut(block.into());
    }
    input[padded - 16..padded - 12]
        .try_into()
        .expect("four-byte tag")
}

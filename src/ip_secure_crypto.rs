//! Shared KNXnet/IP secure wrapper transform (ISO 22510:2019, 5.7).
use aes::{
    Aes128,
    cipher::{BlockEncrypt, KeyInit, KeyIvInit, StreamCipher},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// The one-octet block index permits 255 application blocks (ISO 22510, figure 85).
pub(crate) const MAX_PLAINTEXT: usize = 255 * 16;

fn mac(key: &[u8; 16], first: &[u8; 16], associated: &[u8], plaintext: &[u8]) -> [u8; 16] {
    let mut input = Zeroizing::new(Vec::with_capacity(
        18 + associated.len() + plaintext.len() + 15,
    ));
    input.extend(first);
    input.extend((associated.len() as u16).to_be_bytes());
    input.extend(associated);
    input.extend(plaintext);
    let padded = input.len().next_multiple_of(16);
    input.resize(padded, 0);
    let aes = Aes128::new(key.into());
    let mut state = [0; 16];
    for block in input.as_chunks::<16>().0 {
        for (byte, next) in state.iter_mut().zip(block) {
            *byte ^= next;
        }
        aes.encrypt_block((&mut state).into());
    }
    state
}

fn crypt(key: &[u8; 16], mut counter: [u8; 16], tag: &mut [u8; 16], payload: &mut [u8]) {
    counter[14..].copy_from_slice(&[0xff, 0]);
    let mut stream = ctr::Ctr128BE::<Aes128>::new(key.into(), (&counter).into());
    stream.apply_keystream(tag);
    stream.apply_keystream(payload);
}

#[cfg(feature = "ip-secure")]
pub(crate) fn handshake_tag(key: &[u8; 16], associated: &[u8]) -> [u8; 16] {
    let mut tag = mac(key, &[0; 16], associated, &[]);
    crypt(key, [0; 16], &mut tag, &mut []);
    tag
}

fn first_block(packet: &[u8], payload_len: usize) -> [u8; 16] {
    let mut block = [0; 16];
    block[..14].copy_from_slice(&packet[8..22]);
    block[14..].copy_from_slice(&(payload_len as u16).to_be_bytes());
    block
}

#[cfg(feature = "ip-secure")]
pub(crate) fn seal(
    key: &[u8; 16],
    id: u16,
    sequence: u64,
    serial: [u8; 6],
    plaintext: &[u8],
) -> Option<Vec<u8>> {
    seal_with_tag(key, id, sequence, serial, 0, plaintext)
}

pub(crate) fn seal_with_tag(
    key: &[u8; 16],
    id: u16,
    sequence: u64,
    serial: [u8; 6],
    tag: u16,
    plaintext: &[u8],
) -> Option<Vec<u8>> {
    if sequence >= 1 << 48 || !(6..=MAX_PLAINTEXT).contains(&plaintext.len()) {
        return None;
    }
    let size = (plaintext.len() + 38) as u16;
    let mut packet = Vec::with_capacity(usize::from(size));
    packet.extend([6, 16, 9, 0x50]);
    packet.extend(size.to_be_bytes());
    packet.extend(id.to_be_bytes());
    packet.extend(&sequence.to_be_bytes()[2..]);
    packet.extend(serial);
    packet.extend(tag.to_be_bytes());
    let block = first_block(&packet, plaintext.len());
    let mut tag = mac(key, &block, &packet[..8], plaintext);
    packet.extend(plaintext);
    crypt(key, block, &mut tag, &mut packet[22..]);
    packet.extend(tag);
    Some(packet)
}

/// Return plaintext only after checking the entire envelope and its authentication tag.
pub(crate) fn open(key: &[u8; 16], packet: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let frame = crate::ip_core::IpFrame::parse(packet).ok()?;
    if frame.service != 0x0950 || !(44..=38 + MAX_PLAINTEXT).contains(&packet.len()) {
        return None;
    }
    let (prefix, encoded_tag) = packet.split_at(packet.len() - 16);
    let mut payload = Zeroizing::new(prefix[22..].to_vec());
    let block = first_block(packet, payload.len());
    let mut tag: [u8; 16] = encoded_tag.try_into().ok()?;
    crypt(key, block, &mut tag, &mut payload);
    if bool::from(mac(key, &block, &packet[..8], &payload).ct_eq(&tag)) {
        Some(payload)
    } else {
        None
    }
}

/// ISO 22510 figures 88–89. Timer Notify authenticates the header with empty payload.
#[cfg(feature = "secure-routing")]
pub(crate) fn timer_mac(key: &[u8; 16], header: &[u8; 20]) -> [u8; 16] {
    let mut block = [0; 16];
    block[..14].copy_from_slice(&header[6..]);
    let mut tag = mac(key, &block, &header[..6], &[]);
    crypt(key, block, &mut tag, &mut []);
    tag
}

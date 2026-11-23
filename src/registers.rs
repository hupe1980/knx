//! Byte layouts for DPT 277–284.1200 multi-register meter readings.
//!
//! The layout supplies a fixed-width UTF-8 label and an ordered set of register
//! fields. Use the commissioned ERL/meter profile to interpret each selector,
//! unit and register position before using the counts in an energy calculation.
//!
//! ```
//! use knx::registers::{RegisterLayout, RegisterFields};
//! let layout = RegisterLayout::from_dpt("277.1200").unwrap();
//! let registers = [RegisterFields { count: Some(-10), selector: Some(0) }; 4];
//! let mut buffer = [0; 36];
//! let bytes = layout.encode("ERL-METER01", None, &registers, &mut buffer)?;
//! let reading = layout.decode(bytes)?;
//! assert_eq!(reading.label, "ERL-METER01");
//! assert_eq!(reading.registers().next(), Some(registers[0]));
//! # Ok::<(), knx::CodecError>(())
//! ```

use crate::{
    CodecError,
    text_time::{DateTime, decode_date_time, decode_utf8, encode_date_time},
};

/// One six-byte register. Count and selector validity are independent. Selector codes are profile-specific.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct RegisterFields {
    pub count: Option<i32>,
    pub selector: Option<u8>,
}

impl RegisterFields {
    fn decode(&[a, b, c, d, selector, flags]: &[u8; 6]) -> Self {
        Self {
            count: (flags & 2 == 0).then_some(i32::from_be_bytes([a, b, c, d])),
            selector: (flags & 1 == 0).then_some(selector),
        }
    }
    fn encode(self) -> [u8; 6] {
        let [a, b, c, d] = self.count.unwrap_or_default().to_be_bytes();
        let mask = u8::from(self.count.is_none()) << 1 | u8::from(self.selector.is_none());
        [a, b, c, d, self.selector.unwrap_or_default(), mask]
    }
}

/// Validated fields borrowed from a complete wire payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterReading<'a> {
    pub label: &'a str,
    pub timestamp: Option<DateTime>,
    entries: &'a [[u8; 6]],
}
impl RegisterReading<'_> {
    pub fn registers(&self) -> impl ExactSizeIterator<Item = RegisterFields> + '_ {
        self.entries.iter().map(RegisterFields::decode)
    }
}

/// Select the exact §9.15 or §9.16 layout from the commissioned DPT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterLayout {
    count: usize,
    dated: bool,
}
impl RegisterLayout {
    pub fn from_dpt(id: &str) -> Option<Self> {
        let (count, dated) = match id {
            "277.1200" => (4, false),
            "278.1200" => (5, false),
            "279.1200" => (6, false),
            "280.1200" => (11, false),
            "281.1200" => (4, true),
            "282.1200" => (5, true),
            "283.1200" => (6, true),
            "284.1200" => (11, true),
            _ => return None,
        };
        Some(Self { count, dated })
    }
    pub const fn register_count(self) -> usize {
        self.count
    }
    pub const fn has_timestamp(self) -> bool {
        self.dated
    }
    pub const fn payload_len(self) -> usize {
        12 + self.count * 6 + if self.dated { 8 } else { 0 }
    }

    /// Accept the unambiguous fixed `A[12]` layout: eleven UTF-8 bytes and one terminal NUL.
    /// The chapter gives no separate padding rule for shorter labels.
    pub fn decode(self, payload: &[u8]) -> Result<RegisterReading<'_>, CodecError> {
        if payload.len() != self.payload_len() {
            return Err(CodecError::PayloadLength {
                expected: self.payload_len(),
                actual: payload.len(),
            });
        }
        let (prefix, body) = payload.split_at(if self.dated { 8 } else { 0 });
        let timestamp = self.dated.then(|| decode_date_time(prefix)).transpose()?;
        let label = decode_utf8(&body[..12])?;
        let (entries, _) = body[12..].as_chunks::<6>();
        if entries.iter().any(|entry| entry[5] & 0xfc != 0) {
            return Err(CodecError::InvalidData);
        }
        Ok(RegisterReading {
            label,
            timestamp,
            entries,
        })
    }

    /// Encode a complete block into caller storage. Rejected input leaves the buffer untouched.
    pub fn encode<'a>(
        self,
        label: &str,
        timestamp: Option<DateTime>,
        entries: &[RegisterFields],
        out: &'a mut [u8],
    ) -> Result<&'a [u8], CodecError> {
        if timestamp.is_some() != self.dated
            || entries.len() != self.count
            || label.len() != 11
            || label.as_bytes().contains(&0)
        {
            return Err(CodecError::OutsideRange);
        }
        let stamp = timestamp.map(encode_date_time).transpose()?;
        let expected = self.payload_len();
        let actual = out.len();
        let written = out
            .get_mut(..expected)
            .ok_or(CodecError::PayloadLength { expected, actual })?;
        let (prefix, body) = written.split_at_mut(if self.dated { 8 } else { 0 });
        if let Some(stamp) = stamp {
            prefix.copy_from_slice(&stamp);
        }
        body[..11].copy_from_slice(label.as_bytes());
        body[11] = 0;
        for (dst, value) in body[12..].as_chunks_mut::<6>().0.iter_mut().zip(entries) {
            *dst = value.encode();
        }
        Ok(written)
    }
}

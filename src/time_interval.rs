//! KNX two-octet time periods (§3.8.2) and delta times (§3.9.2).
use crate::{CodecError, codecs::bytes};
use std::ops::RangeInclusive;

/// A commissioned two-byte time interval, expressed to the caller in exact milliseconds.
///
/// The 10 ms, 100 ms and minute subtypes are restricted to parameters and diagnostic
/// data unless a functional-block specification defines their use.
///
/// ```
/// use knx::TimeIntervalDpt;
///
/// let delta = TimeIntervalDpt::from_dpt("8.005").unwrap();
/// assert_eq!(delta.decode_ms(&[0xff, 0xfe]), Ok(-2000));
/// assert_eq!(delta.encode_ms(-2000), Ok([0xff, 0xfe]));
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeIntervalDpt {
    signed: bool,
    tick_ms: i64,
}

impl TimeIntervalDpt {
    /// Select the time base and signedness from an exact commissioned subtype ID.
    pub fn from_dpt(id: &str) -> Option<Self> {
        let (family, subtype) = id.split_once('.')?;
        let signed = match family {
            "7" => false,
            "8" => true,
            _ => return None,
        };
        let tick_ms = match subtype {
            "002" => 1,
            "003" => 10,
            "004" => 100,
            "005" => 1000,
            "006" => 60_000,
            "007" => 3_600_000,
            _ => return None,
        };
        Some(Self { signed, tick_ms })
    }

    /// Milliseconds represented by one wire count.
    pub const fn resolution_ms(self) -> i64 {
        self.tick_ms
    }

    /// Inclusive minimum and maximum interval in milliseconds.
    pub fn range_ms(self) -> RangeInclusive<i64> {
        let (min, max) = if self.signed {
            (i64::from(i16::MIN), i64::from(i16::MAX))
        } else {
            (0, i64::from(u16::MAX))
        };
        min * self.tick_ms..=max * self.tick_ms
    }

    /// Decode exactly two network-order bytes.
    pub fn decode_ms(self, payload: &[u8]) -> Result<i64, CodecError> {
        let word = bytes(payload)?;
        let ticks = if self.signed {
            i64::from(i16::from_be_bytes(word))
        } else {
            i64::from(u16::from_be_bytes(word))
        };
        Ok(ticks * self.tick_ms)
    }

    /// Encode exactly. Out-of-range and in-range but unrepresentable inputs have distinct errors.
    pub fn encode_ms(self, milliseconds: i64) -> Result<[u8; 2], CodecError> {
        if !self.range_ms().contains(&milliseconds) {
            return Err(CodecError::OutsideRange);
        }
        if milliseconds % self.tick_ms != 0 {
            return Err(CodecError::InexactInterval {
                resolution_ms: self.tick_ms,
            });
        }
        let ticks = milliseconds / self.tick_ms;
        if self.signed {
            Ok((ticks as i16).to_be_bytes())
        } else {
            Ok((ticks as u16).to_be_bytes())
        }
    }
}

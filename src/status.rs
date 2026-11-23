//! KNX DPT 6.020 status with mode.
use crate::{CodecError, codecs::bytes};

/// Active mode reported by the point. Its application meaning is device-specific.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusMode {
    Mode0,
    Mode1,
    Mode2,
}

/// Five logical status flags and one active mode (DPT 6.020).
///
/// ```
/// use knx::{StatusMode, StatusWithMode};
///
/// let status = StatusWithMode::decode(&[0x52]).unwrap();
/// assert_eq!(status.set_flags, [true, false, true, false, true]);
/// assert_eq!(status.mode, StatusMode::Mode1);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusWithMode {
    /// Logical flags in A–E order. A set flag is true and occupies a zero wire bit.
    pub set_flags: [bool; 5],
    /// Exactly one of the three modes is active.
    pub mode: StatusMode,
}

impl StatusWithMode {
    /// Decode one byte, rejecting zero or multiple active modes.
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [raw] = bytes(payload)?;
        let mode = match raw & 7 {
            1 => StatusMode::Mode0,
            2 => StatusMode::Mode1,
            4 => StatusMode::Mode2,
            _ => return Err(CodecError::OutsideRange),
        };
        Ok(Self {
            set_flags: std::array::from_fn(|i| raw & (0x80 >> i) == 0),
            mode,
        })
    }

    /// Encode logical flags and mode into one application byte.
    pub fn encode(self) -> [u8; 1] {
        let mut raw = match self.mode {
            StatusMode::Mode0 => 1,
            StatusMode::Mode1 => 2,
            StatusMode::Mode2 => 4,
        };
        for (i, set) in self.set_flags.into_iter().enumerate() {
            if !set {
                raw |= 0x80 >> i;
            }
        }
        [raw]
    }
}

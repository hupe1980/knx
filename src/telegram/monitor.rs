use super::TelegramError;
use std::fmt;

/// Relative interface clock reading at the start of the captured frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorTimestamp {
    Short(u16),
    Extended(u32),
}

/// A passive cEMI L_Busmon.ind capture, including any errors reported by the interface.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct BusMonitorIndication<'a> {
    pub timestamp: MonitorTimestamp,
    status: u8,
    raw: &'a [u8],
}

impl fmt::Debug for BusMonitorIndication<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BusMonitorIndication")
            .field("timestamp", &self.timestamp)
            .field("status", &self.status)
            .field("capture_len", &self.raw.len())
            .finish_non_exhaustive()
    }
}

impl<'a> BusMonitorIndication<'a> {
    /// Read one complete host record containing status, timestamp and 1–254 link octets.
    pub fn parse(frame: &'a [u8]) -> Result<Self, TelegramError> {
        let [message, n, rest @ ..] = frame else {
            return Err(TelegramError::Length);
        };
        if *message != 0x2b {
            return Err(TelegramError::UnsupportedMonitorProfile);
        }
        let (info, raw) = rest
            .split_at_checked(usize::from(*n))
            .ok_or(TelegramError::Length)?;
        if !(1..=254).contains(&raw.len()) {
            return Err(TelegramError::Length);
        }
        let (status, timestamp) = match info {
            [3, 1, s, 4, 2, a, b] | [4, 2, a, b, 3, 1, s] => {
                (*s, MonitorTimestamp::Short(u16::from_be_bytes([*a, *b])))
            }
            [3, 1, s, 6, 4, a, b, c, d] | [6, 4, a, b, c, d, 3, 1, s] => (
                *s,
                MonitorTimestamp::Extended(u32::from_be_bytes([*a, *b, *c, *d])),
            ),
            _ => return Err(TelegramError::UnsupportedMonitorProfile),
        };
        Ok(Self {
            timestamp,
            status,
            raw,
        })
    }

    /// Captured link octets, including the frame check sequence when supplied.
    pub const fn raw_frame(&self) -> &'a [u8] {
        self.raw
    }

    /// Interface sequence number, modulo eight.
    pub const fn sequence(&self) -> u8 {
        self.status & 7
    }

    /// At least one frame or part of a frame was lost.
    pub const fn lost(&self) -> bool {
        self.status & 8 != 0
    }
    pub const fn overflow(&self) -> bool {
        self.status & 0x10 != 0
    }
    pub const fn parity_error(&self) -> bool {
        self.status & 0x20 != 0
    }
    pub const fn bit_error(&self) -> bool {
        self.status & 0x40 != 0
    }
    pub const fn frame_error(&self) -> bool {
        self.status & 0x80 != 0
    }
}

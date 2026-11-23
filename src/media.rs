//! Receive-only group indications from host interfaces. Use the commissioned medium
//! and domain. Each call handles one cEMI record on that interface.
use crate::{
    GroupAddress,
    telegram::{GroupService, GroupTelegram, LinkPrimitive, TelegramError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Medium {
    TwistedPair,
    /// PL110 standard frames with an explicit two-octet domain.
    PowerLine {
        domain: std::num::NonZeroU8,
    },
    /// Asynchronous RF domain-addressed group traffic.
    Radio {
        domain: [u8; 6],
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Metadata {
    TwistedPair {
        timestamp: Option<u16>,
    },
    PowerLine {
        domain: std::num::NonZeroU8,
        timestamp: Option<u16>,
    },
    Radio {
        domain: [u8; 6],
        info: u8,
        frame_number: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Indication<'a> {
    pub source: u16,
    pub destination: GroupAddress,
    /// Routing counter observed on the selected interface.
    pub hop_count: u8,
    pub service: GroupService<'a>,
    pub medium: Metadata,
}

impl Medium {
    pub fn indication(self, frame: &[u8]) -> Result<Indication<'_>, TelegramError> {
        match self {
            Self::TwistedPair => {
                let t = GroupTelegram::parse(frame)?;
                if t.primitive != LinkPrimitive::Indication {
                    return Err(TelegramError::UnsupportedProfile);
                }
                Ok(Indication {
                    source: t.source,
                    destination: t.destination,
                    hop_count: t.hop_count(),
                    service: t.service,
                    medium: Metadata::TwistedPair {
                        timestamp: t.relative_timestamp,
                    },
                })
            }
            Self::PowerLine { domain } => {
                let [0x29, metadata_len, payload @ ..] = frame else {
                    return Err(TelegramError::UnsupportedProfile);
                };
                let (metadata, body) = payload
                    .split_at_checked(usize::from(*metadata_len))
                    .ok_or(TelegramError::Length)?;
                let (wire_domain, timestamp) = match metadata {
                    [1, 2, a, b] => ([*a, *b], None),
                    [1, 2, a, b, 4, 2, hi, lo] | [4, 2, hi, lo, 1, 2, a, b] => {
                        ([*a, *b], Some(u16::from_be_bytes([*hi, *lo])))
                    }
                    _ => return Err(TelegramError::UnsupportedProfile),
                };
                if wire_domain != [0, domain.get()] {
                    return Err(TelegramError::UnsupportedProfile);
                }
                if body.first().is_some_and(|ctrl| ctrl & 0x80 == 0) {
                    return Err(TelegramError::UnsupportedProfile);
                }
                let telegram = GroupTelegram::parse_body(0x29, timestamp, body)?;
                // PL110 reserves the top group-address bit (main groups 0–15).
                if u16::from(telegram.destination) > 0x7fff {
                    return Err(TelegramError::UnsupportedProfile);
                }
                Ok(Indication {
                    hop_count: telegram.hop_count(),
                    source: telegram.source,
                    destination: telegram.destination,
                    service: telegram.service,
                    medium: Metadata::PowerLine { domain, timestamp },
                })
            }
            Self::Radio { domain } => {
                let [0x29, 10, 2, 8, info, rf_domain @ ..] = frame else {
                    return Err(TelegramError::UnsupportedProfile);
                };
                let (wire_domain, body) =
                    rf_domain.split_at_checked(6).ok_or(TelegramError::Length)?;
                let [lfn, body @ ..] = body else {
                    return Err(TelegramError::Length);
                };
                if wire_domain != domain || *lfn > 7 || body.first().is_some_and(|c| c & 0x80 != 0)
                {
                    return Err(TelegramError::UnsupportedProfile);
                }
                let telegram = GroupTelegram::parse_body(0x29, None, body)?;
                Ok(Indication {
                    source: telegram.source,
                    destination: telegram.destination,
                    hop_count: telegram.hop_count(),
                    service: telegram.service,
                    medium: Metadata::Radio {
                        domain,
                        info: *info,
                        frame_number: *lfn,
                    },
                })
            }
        }
    }
}

/// Borrowed group filter bitmap from a commissioned coupler in filter mode.
/// Legacy tables cover main groups 0–13. Full tables cover all 32 main groups.
/// The caller selects the table for the actual interface and direction.
#[derive(Clone, Copy)]
pub struct GroupFilterTable<'a>(&'a [u8]);

impl<'a> GroupFilterTable<'a> {
    /// Accept a complete 3,584- or 8,192-octet bitmap.
    pub fn from_bytes(bitmap: &'a [u8]) -> Option<Self> {
        matches!(bitmap.len(), 3584 | 8192).then_some(Self(bitmap))
    }

    /// Inspect the commissioned entry. `None` means this address lies outside the table.
    /// A set bit is configuration evidence. Observing delivery requires an egress capture.
    pub fn entry(self, address: GroupAddress) -> Option<bool> {
        let raw = usize::from(u16::from(address));
        self.0.get(raw / 8).map(|byte| byte & (1 << (raw % 8)) != 0)
    }
}

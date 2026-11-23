//! Decode and encode cEMI group telegrams or inspect a passive bus-monitor capture.
//!
//! Pass one cEMI record after removing its transport envelope. Use the commissioned
//! DPT to interpret a value. A group read and a link confirmation carry no observation.
//! For link errors and relative capture timestamps, use [`BusMonitorIndication`].
//!
//! ```
//! use knx::{decode_switch, telegram::GroupTelegram};
//! let frame = [0x29, 0, 0xbc, 0xe0, 0xff, 0xff, 0x23, 2, 1, 0, 0x81];
//! let telegram = GroupTelegram::parse(&frame)?;
//! assert_eq!(telegram.destination.to_string(), "4/3/2");
//! assert_eq!(decode_switch(telegram.indicated_value().unwrap().as_bytes()), Ok(true));
//! # Ok::<(), knx::telegram::TelegramError>(())
//! ```

mod monitor;
pub use monitor::{BusMonitorIndication, MonitorTimestamp};

use crate::GroupAddress;
use std::fmt;

/// Link primitive and, for a confirmation, its link-layer outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkPrimitive {
    Request,
    Indication,
    Confirmation { successful: bool },
}

/// Application data in the low six bits or in separate octets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupData<'a> {
    Inline(u8),
    Bytes(&'a [u8]),
}

impl GroupData<'_> {
    /// Extracted application payload for a commissioned DPT decoder.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Inline(value) => std::slice::from_ref(value),
            Self::Bytes(bytes) => bytes,
        }
    }
}

/// Connectionless group application service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupService<'a> {
    Read,
    Response(GroupData<'a>),
    Write(GroupData<'a>),
}

/// A group telegram with optional relative timestamp metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupTelegram<'a> {
    pub primitive: LinkPrimitive,
    /// Individual source address in network notation (area, line, device).
    pub source: u16,
    pub destination: GroupAddress,
    pub service: GroupService<'a>,
    /// Raw two-octet relative timestamp from cEMI additional information.
    pub relative_timestamp: Option<u16>,
    /// Link repeat, priority and acknowledgement flags, preserved when forwarding bytes.
    flags: u8,
    hops: u8,
    // Preserve an incoming extended frame even when its payload fits a standard frame.
    short_extended: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TelegramError {
    Length,
    UnsupportedProfile,
    UnsupportedMonitorProfile,
    InvalidService,
    InvalidPayload,
    OutputTooSmall,
}

impl fmt::Display for TelegramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Length => "Supply one complete cEMI frame with a valid length.",
            Self::UnsupportedProfile => "Use an ordinary cEMI group frame with supported metadata.",
            Self::UnsupportedMonitorProfile => {
                "Supply one L_Busmon.ind record with status and a relative timestamp."
            }
            Self::InvalidService => "Use a connectionless group read, response or write.",
            Self::InvalidPayload => "Check the inline bits or supply 1 to 253 application octets.",
            Self::OutputTooSmall => "Provide enough output space for the complete cEMI frame.",
        })
    }
}
impl std::error::Error for TelegramError {}

impl<'a> GroupTelegram<'a> {
    /// Largest supported cEMI record, including a relative timestamp.
    pub const MAX_ENCODED_LEN: usize = 11 + 4 + 253;

    /// Construct a normal group request with low priority and six routing hops.
    pub fn request(source: u16, destination: GroupAddress, service: GroupService<'a>) -> Self {
        Self {
            primitive: LinkPrimitive::Request,
            source,
            destination,
            service,
            relative_timestamp: None,
            flags: 0x3c,
            short_extended: false,
            hops: 6,
        }
    }

    /// Routing counter carried by this cEMI frame.
    pub const fn hop_count(&self) -> u8 {
        self.hops
    }

    /// Parse one standard or extended cEMI record. Application octets borrow the supplied slice.
    pub fn parse(frame: &'a [u8]) -> Result<Self, TelegramError> {
        let [message, info_len, rest @ ..] = frame else {
            return Err(TelegramError::Length);
        };
        let (info, body) = rest
            .split_at_checked(usize::from(*info_len))
            .ok_or(TelegramError::Length)?;
        let relative_timestamp = match info {
            [] => None,
            [4, 2, high, low] => Some(u16::from_be_bytes([*high, *low])),
            _ => return Err(TelegramError::UnsupportedProfile),
        };
        Self::parse_body(*message, relative_timestamp, body)
    }

    pub(crate) fn parse_body(
        message: u8,
        relative_timestamp: Option<u16>,
        body: &'a [u8],
    ) -> Result<Self, TelegramError> {
        let [
            control,
            routing,
            sh,
            sl,
            dh,
            dl,
            length,
            transport,
            apci,
            data @ ..,
        ] = body
        else {
            return Err(TelegramError::Length);
        };
        let standard = control & 0x80 != 0;
        if *length == 255 || (standard && *length > 15) || usize::from(*length) != data.len() + 1 {
            return Err(TelegramError::Length);
        }
        // Ordinary group addressing uses the zero extended-frame format.
        if control & 0x50 != 0x10 || routing & 0x8f != 0x80 {
            return Err(TelegramError::UnsupportedProfile);
        }
        let primitive = match message {
            0x11 if control & 1 == 0 => LinkPrimitive::Request,
            0x29 if control & 1 == 0 => LinkPrimitive::Indication,
            0x2e => LinkPrimitive::Confirmation {
                successful: control & 1 == 0,
            },
            _ => return Err(TelegramError::UnsupportedProfile),
        };
        let destination = GroupAddress::try_from(u16::from_be_bytes([*dh, *dl]))
            .map_err(|_| TelegramError::UnsupportedProfile)?;
        let service = decode_service(*transport, *apci, data)?;
        Ok(Self {
            primitive,
            destination,
            source: u16::from_be_bytes([*sh, *sl]),
            service,
            relative_timestamp,
            flags: control & 0x7e,
            short_extended: !standard && *length <= 15,
            hops: (routing >> 4) & 7,
        })
    }

    /// Extract incoming value data, excluding requests, reads and confirmations.
    pub fn indicated_value(&self) -> Option<GroupData<'a>> {
        match (self.primitive, self.service) {
            (
                LinkPrimitive::Indication,
                GroupService::Response(data) | GroupService::Write(data),
            ) => Some(data),
            _ => None,
        }
    }

    /// Write one cEMI record and return its length. Errors leave the buffer untouched.
    pub fn encode(&self, output: &mut [u8]) -> Result<usize, TelegramError> {
        let (service, payload) = match self.service {
            GroupService::Read => (0, GroupData::Inline(0)),
            GroupService::Response(value) => (0x40, value),
            GroupService::Write(value) => (0x80, value),
        };
        let (inline, data): (_, &[u8]) = match payload {
            GroupData::Inline(bits) if bits <= 63 => (bits, &[]),
            GroupData::Bytes(bytes) if (1..=253).contains(&bytes.len()) => (0, bytes),
            _ => return Err(TelegramError::InvalidPayload),
        };
        let info_len = if self.relative_timestamp.is_some() {
            4
        } else {
            0
        };
        let len = 11 + info_len + data.len();
        let frame = output.get_mut(..len).ok_or(TelegramError::OutputTooSmall)?;
        let (message, confirm_error) = match self.primitive {
            LinkPrimitive::Request => (0x11, false),
            LinkPrimitive::Indication => (0x29, false),
            LinkPrimitive::Confirmation { successful } => (0x2e, !successful),
        };
        frame[0..2].copy_from_slice(&[message, info_len as u8]);
        if let Some(ticks) = self.relative_timestamp {
            let [high, low] = ticks.to_be_bytes();
            frame[2..6].copy_from_slice(&[4, 2, high, low]);
        }
        let body = &mut frame[2 + info_len..];
        let standard = data.len() <= 14 && !self.short_extended;
        body[0] = self.flags | (u8::from(standard) << 7) | u8::from(confirm_error);
        body[1] = 0x80 | (self.hops << 4);
        body[2..4].copy_from_slice(&self.source.to_be_bytes());
        body[4..6].copy_from_slice(&u16::from(self.destination).to_be_bytes());
        body[6..9].copy_from_slice(&[1 + data.len() as u8, 0, service | inline]);
        body[9..].copy_from_slice(data);
        Ok(len)
    }
}

pub(crate) fn decode_service(
    transport: u8,
    apci: u8,
    data: &[u8],
) -> Result<GroupService<'_>, TelegramError> {
    if transport != 0 || apci >> 6 == 3 {
        return Err(TelegramError::InvalidService);
    }
    let inline = apci & 0x3f;
    let payload = match data {
        [] => GroupData::Inline(inline),
        _ if inline == 0 => GroupData::Bytes(data),
        _ => return Err(TelegramError::InvalidPayload),
    };
    let service = match apci >> 6 {
        0 if data.is_empty() && inline == 0 => GroupService::Read,
        1 => GroupService::Response(payload),
        2 => GroupService::Write(payload),
        _ => return Err(TelegramError::InvalidPayload),
    };
    Ok(service)
}

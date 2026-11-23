//! Classic remote property-description service on an individual application connection.
use std::{fmt, num::NonZeroU8};

/// A correlated remote property-description read. The transport owns the individual connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PropertyDescriptionRead {
    peer: u16,
    object: u8,
    property: u8,
    index: u8,
}

/// Metadata advertised by a KNX interface object. Access levels describe the device's
/// policy. A caller still needs authorization to perform the corresponding operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PropertyDescription {
    pub object_index: u8,
    pub property_id: NonZeroU8,
    pub property_index: u8,
    pub data_type: u8,
    pub write_enabled: bool,
    pub max_elements: u16,
    pub read_level: u8,
    pub write_level: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DescriptionError {
    Mismatch,
    Unavailable,
    UnsupportedFormat,
}
impl fmt::Display for DescriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mismatch => "Match the response to the originating device and property query.",
            Self::Unavailable => "The device returned no property description.",
            Self::UnsupportedFormat => {
                "Use a complete classic property description with a 12-bit element count."
            }
        })
    }
}
impl std::error::Error for DescriptionError {}

impl PropertyDescriptionRead {
    /// Look up a specific PID. The device supplies its index in the result.
    pub const fn by_id(peer: u16, object: u8, property: NonZeroU8) -> Self {
        Self {
            peer,
            object,
            property: property.get(),
            index: 0,
        }
    }
    /// Select by zero-based property index. The device supplies its PID in the result.
    pub const fn by_index(peer: u16, object: u8, index: u8) -> Self {
        Self {
            peer,
            object,
            property: 0,
            index,
        }
    }
    /// A_PropertyDescription_Read APDU, with transport bits clear.
    pub const fn request(self) -> [u8; 5] {
        [3, 0xd8, self.object, self.property, self.index]
    }

    /// Interpret an A_PropertyDescription_Response APDU from the actual individual peer.
    pub fn response(self, peer: u16, apdu: &[u8]) -> Result<PropertyDescription, DescriptionError> {
        let [3, 0xd9, object, id, index, kind, hi, lo, level] = *apdu else {
            return Err(DescriptionError::UnsupportedFormat);
        };
        let selected = if self.property == 0 {
            index == self.index
        } else {
            id == self.property
        };
        if peer != self.peer || object != self.object || !selected {
            return Err(DescriptionError::Mismatch);
        }
        let max_elements = u16::from_be_bytes([hi, lo]);
        if max_elements == 0 {
            return Err(DescriptionError::Unavailable);
        }
        if hi > 15 || kind & 0x40 != 0 {
            return Err(DescriptionError::UnsupportedFormat);
        }
        let property_id = NonZeroU8::new(id).ok_or(DescriptionError::UnsupportedFormat)?;
        Ok(PropertyDescription {
            object_index: object,
            property_id,
            property_index: index,
            data_type: kind & 63,
            write_enabled: kind & 128 != 0,
            max_elements,
            read_level: level >> 4,
            write_level: level & 15,
        })
    }
}

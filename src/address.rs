use std::{fmt, num::NonZeroU16, str::FromStr};

fn decimal(text: &str, max: u16) -> Option<u16> {
    if text.len() > 1 && text.starts_with('0') || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u16>().ok().filter(|value| *value <= max)
}

fn parse_raw(text: &str) -> Option<u16> {
    let mut parts = text.split('/');
    let first = parts.next()?;
    let value = match (parts.next(), parts.next(), parts.next()) {
        (None, None, None) => decimal(first, u16::MAX)?,
        (Some(sub), None, None) => decimal(first, 31)? << 11 | decimal(sub, 2047)?,
        (Some(middle), Some(sub), None) => {
            decimal(first, 31)? << 11 | decimal(middle, 7)? << 8 | decimal(sub, 255)?
        }
        _ => return None,
    };
    Some(value)
}

/// A nonzero KNX group destination, independent of its ETS display style.
///
/// Parse canonical unsigned decimal components in three-level, two-level or
/// raw notation. Display always writes the equivalent three-level address.
///
/// ```
/// use knx::GroupAddress;
///
/// let address: GroupAddress = "2/259".parse()?;
/// assert_eq!(address.to_string(), "2/1/3");
/// assert_eq!(u16::from(address), 4355);
/// assert_eq!(address, GroupAddress::try_from(4355)?);
/// # Ok::<(), knx::InvalidGroupAddress>(())
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct GroupAddress(NonZeroU16);

/// An invalid ETS group address or the reserved numeric address zero.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct InvalidGroupAddress;

impl fmt::Display for InvalidGroupAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "Use a nonzero ETS group address in three-level, two-level or decimal notation.",
        )
    }
}

impl std::error::Error for InvalidGroupAddress {}

impl FromStr for GroupAddress {
    type Err = InvalidGroupAddress;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from(parse_raw(text).ok_or(InvalidGroupAddress)?)
    }
}

impl TryFrom<&str> for GroupAddress {
    type Error = InvalidGroupAddress;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl TryFrom<u16> for GroupAddress {
    type Error = InvalidGroupAddress;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        NonZeroU16::new(value).map(Self).ok_or(InvalidGroupAddress)
    }
}

impl From<GroupAddress> for u16 {
    fn from(address: GroupAddress) -> Self {
        address.0.get()
    }
}

impl fmt::Display for GroupAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = self.0.get();
        write!(f, "{}/{}/{}", raw >> 11, (raw >> 8) & 7, raw & 255)
    }
}

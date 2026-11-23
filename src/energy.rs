//! Signed electrical energy registers in their commissioned units.
use crate::{CodecError, codecs::bytes};

/// Four-byte electrical energy subtypes in KNX §3.14.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Energy32 {
    /// DPT 13.010, active watt-hours.
    ActiveWh,
    /// DPT 13.011, apparent volt-ampere-hours.
    ApparentVah,
    /// DPT 13.012, reactive volt-ampere-reactive-hours.
    ReactiveVarh,
    /// DPT 13.013, active kilowatt-hours.
    ActiveKwh,
    /// DPT 13.014, apparent kilovolt-ampere-hours.
    ApparentKvah,
    /// DPT 13.015, reactive kilovolt-ampere-reactive-hours.
    ReactiveKvarh,
    /// DPT 13.016, active megawatt-hours.
    ActiveMwh,
}

impl Energy32 {
    /// Look up the exact commissioned DPT identifier.
    pub fn from_dpt(id: &str) -> Option<Self> {
        Some(match id {
            "13.010" => Self::ActiveWh,
            "13.011" => Self::ApparentVah,
            "13.012" => Self::ReactiveVarh,
            "13.013" => Self::ActiveKwh,
            "13.014" => Self::ApparentKvah,
            "13.015" => Self::ReactiveKvarh,
            "13.016" => Self::ActiveMwh,
            _ => return None,
        })
    }

    /// Unit of one integer count on the wire.
    pub const fn unit(self) -> &'static str {
        match self {
            Self::ActiveWh => "Wh",
            Self::ApparentVah => "VAh",
            Self::ReactiveVarh => "VARh",
            Self::ActiveKwh => "kWh",
            Self::ApparentKvah => "kVAh",
            Self::ReactiveKvarh => "kVARh",
            Self::ActiveMwh => "MWh",
        }
    }

    /// Decode exactly four application bytes to whole units.
    pub fn decode(self, payload: &[u8]) -> Result<i32, CodecError> {
        Ok(i32::from_be_bytes(bytes(payload)?))
    }

    /// Encode a signed count in the selected unit.
    pub const fn encode(self, count: i32) -> [u8; 4] {
        count.to_be_bytes()
    }
}

/// Eight-byte electrical energy subtypes in KNX §3.28.1.
///
/// Select the register using the commissioned subtype, then decode the extracted bytes.
///
/// ```
/// use knx::{Energy32, Energy64};
///
/// let register = Energy64::from_dpt("29.011").unwrap();
/// assert_eq!(register.companion(), Energy32::ApparentVah);
/// assert_eq!(register.decode(&[0, 32, 0, 0, 0, 0, 0, 1]), Ok(9_007_199_254_740_993));
/// assert_eq!(register.unit(), "VAh");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Energy64 {
    /// DPT 29.010, active energy in Wh.
    ActiveWh,
    /// DPT 29.011, apparent energy in VAh.
    ApparentVah,
    /// DPT 29.012, reactive energy in VARh.
    ReactiveVarh,
}

impl Energy64 {
    /// Look up the exact commissioned DPT identifier.
    pub fn from_dpt(id: &str) -> Option<Self> {
        Some(match id {
            "29.010" => Self::ActiveWh,
            "29.011" => Self::ApparentVah,
            "29.012" => Self::ReactiveVarh,
            _ => return None,
        })
    }

    /// Four-byte datapoint required alongside this subtype by §3.28.1.
    pub const fn companion(self) -> Energy32 {
        match self {
            Self::ActiveWh => Energy32::ActiveWh,
            Self::ApparentVah => Energy32::ApparentVah,
            Self::ReactiveVarh => Energy32::ReactiveVarh,
        }
    }

    /// Unit of one integer count on the wire.
    pub const fn unit(self) -> &'static str {
        self.companion().unit()
    }

    /// Decode exactly eight application bytes to whole units.
    pub fn decode(self, payload: &[u8]) -> Result<i64, CodecError> {
        Ok(i64::from_be_bytes(bytes(payload)?))
    }

    /// Encode a signed count in the selected unit.
    pub const fn encode(self, count: i64) -> [u8; 8] {
        count.to_be_bytes()
    }
}

//! Two-bit priority controls as defined in KNX Datapoint Types §3.2.
use crate::{CodecError, codecs::bytes};

/// Requested priority override for a commissioned binary function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriorityControl {
    /// Relinquish control. The resulting actuator state depends on its configuration.
    Release,
    /// Apply the zero or one function value defined by the commissioned subtype.
    Set(bool),
}

/// Commissioned two-bit subtype. Each uses the same layout with its own function labels.
///
/// ```
/// use knx::{PriorityControl, PriorityDpt};
///
/// let switch = PriorityDpt::from_dpt("2.001").unwrap();
/// assert_eq!(switch.decode(&[1]), Ok(PriorityControl::Release));
/// assert_eq!(switch.encode(PriorityControl::Set(false)), [2]);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriorityDpt {
    /// DPT 2.001, off or on.
    Switch,
    /// DPT 2.002, false or true.
    Boolean,
    /// DPT 2.003, disable or enable.
    Enable,
    /// DPT 2.004, no-ramp or ramp.
    Ramp,
    /// DPT 2.005, no-alarm or alarm.
    Alarm,
    /// DPT 2.006, low or high.
    BinaryValue,
    /// DPT 2.007, decrease or increase.
    Step,
    /// DPT 2.008, up or down.
    UpDown,
    /// DPT 2.009, open or close.
    OpenClose,
    /// DPT 2.010, stop or start.
    Start,
    /// DPT 2.011, inactive or active.
    State,
    /// DPT 2.012, normal or inverted.
    Invert,
}

impl PriorityDpt {
    /// Look up an exact DPT identifier in this family.
    pub fn from_dpt(id: &str) -> Option<Self> {
        Some(match id {
            "2.001" => Self::Switch,
            "2.002" => Self::Boolean,
            "2.003" => Self::Enable,
            "2.004" => Self::Ramp,
            "2.005" => Self::Alarm,
            "2.006" => Self::BinaryValue,
            "2.007" => Self::Step,
            "2.008" => Self::UpDown,
            "2.009" => Self::OpenClose,
            "2.010" => Self::Start,
            "2.011" => Self::State,
            "2.012" => Self::Invert,
            _ => return None,
        })
    }

    /// Function labels in wire order (zero, one).
    pub const fn labels(self) -> (&'static str, &'static str) {
        match self {
            Self::Switch => ("off", "on"),
            Self::Boolean => ("false", "true"),
            Self::Enable => ("disable", "enable"),
            Self::Ramp => ("no-ramp", "ramp"),
            Self::Alarm => ("no-alarm", "alarm"),
            Self::BinaryValue => ("low", "high"),
            Self::Step => ("decrease", "increase"),
            Self::UpDown => ("up", "down"),
            Self::OpenClose => ("open", "close"),
            Self::Start => ("stop", "start"),
            Self::State => ("inactive", "active"),
            Self::Invert => ("normal", "inverted"),
        }
    }

    /// Decode two right-aligned bits in a single extracted application byte.
    pub fn decode(self, payload: &[u8]) -> Result<PriorityControl, CodecError> {
        match bytes(payload)? {
            [0 | 1] => Ok(PriorityControl::Release),
            [2] => Ok(PriorityControl::Set(false)),
            [3] => Ok(PriorityControl::Set(true)),
            _ => Err(CodecError::OutsideRange),
        }
    }

    /// Encode the command for subsequent framing. Release uses the canonical zero byte.
    pub const fn encode(self, command: PriorityControl) -> [u8; 1] {
        [match command {
            PriorityControl::Release => 0,
            PriorityControl::Set(false) => 2,
            PriorityControl::Set(true) => 3,
        }]
    }
}

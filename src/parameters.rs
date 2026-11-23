//! Commissioning parameter types from KNX Interworking v02.02.01 §11.
//!
//! Look up a standard parameter by its hexadecimal identifier. Validate the
//! extracted, right-aligned value before storing it. The returned bytes preserve
//! the original DPT encoding, including equivalent floating-point encodings.
//!
//! ```
//! use knx::{parameters::ParameterType, structured::encode_time_delay_seconds};
//! let cycle = ParameterType::from_id(0x1401).unwrap();
//! let seconds = encode_time_delay_seconds(30)?;
//! assert_eq!(cycle.validate(&seconds)?.as_bytes(), &[8]);
//! assert!(cycle.validate(&[0]).is_err()); // Zero is only valid for other delay parameters.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
use crate::{CodecError, Float16Dpt, HvacMode, codecs::bytes, hvac, structured, text_time};
use std::fmt;

/// Degree of validation possible from the parameter and its referenced type.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Validation {
    Value,
    /// Width and explicitly assigned bit fields are known. Other semantics are unresolved.
    Layout,
    /// The edition leaves the parameter wire format undetermined.
    Unspecified,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Binding {
    Datapoint(&'static str),
    /// The edition refers to a placeholder or a subtype without a definition in this edition.
    Unresolved(&'static str),
    None,
}

/// An assigned or named entry in the commissioning parameter namespace.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ParameterType {
    name: &'static str,
    id: Option<u16>,
    binding: Binding,
    rule: Rule,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Rule {
    Bit,
    Scene,
    DateTime,
    Cycle,
    Delay,
    Prewarning,
    Hvac,
    Role,
    Adaptive,
    SwitchAction,
    AlarmReaction,
    TravelAction,
    HvacAction,
    TwoBits,
    Byte,
    Word,
    Float16(&'static str),
    FourBits,
    Unspecified,
}

macro_rules! parameters {
    ($(($name:literal, $id:expr, $binding:expr, $rule:expr)),+ $(,)?) => {
        /// All 32 assigned and four unnumbered parameters in this edition.
        pub const PARAMETERS: &[ParameterType] = &[
            $(ParameterType{name:concat!("PART_", $name),id:$id,binding:$binding,rule:$rule},)+
        ];
    };
}

use Binding::{Datapoint, Unresolved};
parameters!(
    ("Switch_Value", Some(0x0101), Datapoint("1.001"), Rule::Bit),
    ("Boolean", Some(0x0102), Datapoint("1.002"), Rule::Bit),
    ("UpDown_Action", Some(0x0103), Datapoint("1.008"), Rule::Bit),
    ("Invert", Some(0x0104), Datapoint("1.012"), Rule::Bit),
    ("Logical", Some(0x0105), Datapoint("1.021"), Rule::Bit),
    ("Scene_Value", Some(0x0106), Datapoint("1.022"), Rule::Bit),
    ("Blind_Mode", Some(0x0107), Datapoint("1.023"), Rule::Bit),
    ("Enable", Some(0x0108), Datapoint("1.003"), Rule::Bit),
    (
        "Scene_Number",
        Some(0x1101),
        Datapoint("17.001"),
        Rule::Scene
    ),
    (
        "Date_Time",
        Some(0x1301),
        Datapoint("19.001"),
        Rule::DateTime
    ),
    ("Cycle_Time", Some(0x1401), Datapoint("20.013"), Rule::Cycle),
    ("Time_Delay", Some(0x1402), Datapoint("20.013"), Rule::Delay),
    (
        "Prewarning_Delay",
        Some(0x1403),
        Datapoint("20.013"),
        Rule::Prewarning
    ),
    ("HVACMode", Some(0x1404), Datapoint("20.102"), Rule::Hvac),
    (
        "MasterSlaveMode",
        Some(0x1405),
        Datapoint("20.112"),
        Rule::Role
    ),
    (
        "Adaptive_Selection",
        Some(0xe401),
        Unresolved("228.1000"),
        Rule::Adaptive
    ),
    (
        "OnOff_Action",
        Some(0x1701),
        Datapoint("23.001"),
        Rule::SwitchAction
    ),
    (
        "Alarm_Reaction",
        Some(0x1702),
        Datapoint("23.002"),
        Rule::AlarmReaction
    ),
    (
        "UpDown_Switch_Action",
        Some(0x1703),
        Datapoint("23.003"),
        Rule::TravelAction
    ),
    (
        "PB_Action_HVAC",
        Some(0x1704),
        Datapoint("23.102"),
        Rule::HvacAction
    ),
    (
        "UpDown_Action_Extended",
        Some(0x1705),
        Unresolved("23.xxx"),
        Rule::TwoBits
    ),
    ("Byte_Value", Some(0x0501), Binding::None, Rule::Byte),
    (
        "Dimming_Value",
        Some(0x0502),
        Datapoint("5.001"),
        Rule::Byte
    ),
    (
        "Adjustable_Selection",
        Some(0x0503),
        Datapoint("5.010"),
        Rule::Byte
    ),
    (
        "Shutter_Position",
        Some(0x0504),
        Datapoint("5.001"),
        Rule::Byte
    ),
    (
        "Slat_Position",
        Some(0x0505),
        Datapoint("5.001"),
        Rule::Byte
    ),
    ("Orientation", Some(0x0506), Datapoint("5.003"), Rule::Byte),
    ("Render_Value", Some(0x0701), Datapoint("7.001"), Rule::Word),
    ("Light_Value", Some(0x0702), Datapoint("7.013"), Rule::Word),
    (
        "Move_UpDown_Time",
        Some(0x0703),
        Datapoint("7.005"),
        Rule::Word
    ),
    (
        "COV_Lux",
        Some(0x0901),
        Datapoint("9.004"),
        Rule::Float16("9.004")
    ),
    (
        "Value_Tempd",
        Some(0x0902),
        Datapoint("9.002"),
        Rule::Float16("9.002")
    ),
    ("Input_Connected", None, Binding::None, Rule::FourBits),
    (
        "PB_Action_HVAC_Extended",
        None,
        Binding::None,
        Rule::Unspecified
    ),
    ("COV_Power", None, Datapoint("14.056"), Rule::Unspecified),
    ("COV_Energy", None, Datapoint("13.010"), Rule::Unspecified),
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ParameterError {
    Value(CodecError),
    UnspecifiedFormat,
}
impl fmt::Display for ParameterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(error) => error.fmt(f),
            Self::UnspecifiedFormat => {
                f.write_str("The parameter wire format is unspecified in this edition.")
            }
        }
    }
}
impl std::error::Error for ParameterError {}
impl From<CodecError> for ParameterError {
    fn from(e: CodecError) -> Self {
        Self::Value(e)
    }
}

/// A value checked to the extent indicated by its parameter's validation scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParameterPayload<'a> {
    parameter: ParameterType,
    bytes: &'a [u8],
}
impl<'a> ParameterPayload<'a> {
    pub fn parameter(self) -> ParameterType {
        self.parameter
    }
    pub fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }
}

impl ParameterType {
    pub fn from_id(id: u16) -> Option<Self> {
        PARAMETERS.iter().copied().find(|p| p.id == Some(id))
    }
    pub fn from_name(name: &str) -> Option<Self> {
        PARAMETERS.iter().copied().find(|p| p.name == name)
    }
    pub fn id(self) -> Option<u16> {
        self.id
    }
    pub fn name(self) -> &'static str {
        self.name
    }
    pub fn binding(self) -> Binding {
        self.binding
    }
    pub fn validation(self) -> Validation {
        match self.rule {
            Rule::Unspecified => Validation::Unspecified,
            Rule::Adaptive | Rule::TwoBits => Validation::Layout,
            _ => Validation::Value,
        }
    }
    /// Check the extracted payload. Storage offsets and packing multiple
    /// parameters into a device's memory layout belong to the device profile.
    pub fn validate(self, data: &[u8]) -> Result<ParameterPayload<'_>, ParameterError> {
        match self.rule {
            Rule::Unspecified => return Err(ParameterError::UnspecifiedFormat),
            Rule::DateTime => {
                text_time::decode_date_time(data)?;
            }
            Rule::Hvac => {
                HvacMode::decode(data)?;
            }
            Rule::Role => {
                hvac::ControllerRole::decode(data)?;
            }
            Rule::SwitchAction => {
                structured::SwitchAction::decode(data)?;
            }
            Rule::AlarmReaction => {
                structured::AlarmReaction::decode(data)?;
            }
            Rule::TravelAction => {
                structured::TravelAction::decode(data)?;
            }
            Rule::HvacAction => {
                hvac::PushButtonAction::decode(data)?;
            }
            Rule::Delay => {
                structured::decode_time_delay_seconds(data)?;
            }
            Rule::Float16(id) => {
                Float16Dpt::from_dpt(id)
                    .expect("registered parameter binding")
                    .decode(data)?;
            }
            Rule::Word => {
                bytes::<2>(data)?;
            }
            Rule::Bit => check_octet(data, |v| v <= 1)?,
            Rule::Scene => check_octet(data, |v| v <= 7)?,
            Rule::Cycle => check_octet(data, |v| matches!(v, 5 | 8 | 9 | 10 | 13 | 15))?,
            Rule::Prewarning => check_octet(data, |v| matches!(v, 0 | 6 | 8 | 10))?,
            Rule::Adaptive => check_octet(data, |v| matches!(v & 7, 1..=3))?,
            Rule::TwoBits => check_octet(data, |v| v <= 3)?,
            Rule::FourBits => check_octet(data, |v| v <= 15)?,
            Rule::Byte => {
                bytes::<1>(data)?;
            }
        }
        Ok(ParameterPayload {
            parameter: self,
            bytes: data,
        })
    }
}

fn check_octet(data: &[u8], allowed: impl FnOnce(u8) -> bool) -> Result<(), CodecError> {
    let [code] = bytes(data)?;
    if allowed(code) {
        Ok(())
    } else {
        Err(CodecError::OutsideRange)
    }
}

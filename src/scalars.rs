//! Additional common scalar datapoints from KNX Interworking v02.02.01.
use crate::{CodecError, codecs::bytes};
use std::num::NonZeroU16;

/// A commissioned one-bit function (§3.1). Decode an extracted bit to its semantic label.
/// Both wire values of 1.017 denote the same trigger. Its canonical encoding is zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryDpt {
    labels: [&'static str; 2],
}

impl BinaryDpt {
    /// Look up an exact identifier and its zero/one labels.
    pub fn from_dpt(id: &str) -> Option<Self> {
        let labels = match id {
            "1.001" => ["off", "on"],
            "1.002" => ["false", "true"],
            "1.003" => ["disable", "enable"],
            "1.004" => ["no-ramp", "ramp"],
            "1.005" => ["no-alarm", "alarm"],
            "1.006" => ["low", "high"],
            "1.007" => ["decrease", "increase"],
            "1.008" => ["up", "down"],
            "1.009" => ["open", "close"],
            "1.010" => ["stop", "start"],
            "1.011" => ["inactive", "active"],
            "1.012" => ["normal", "inverted"],
            "1.013" => ["start-stop", "cyclic"],
            "1.014" => ["fixed", "calculated"],
            "1.015" => ["no-action", "reset"],
            "1.016" => ["no-action", "acknowledge"],
            "1.017" => ["trigger", "trigger"],
            "1.018" => ["vacant", "occupied"],
            "1.019" => ["closed", "open"],
            "1.021" => ["or", "and"],
            "1.022" => ["scene-1", "scene-2"],
            "1.023" => ["shutter", "blind"],
            "1.024" => ["day", "night"],
            _ => return None,
        };
        Some(Self { labels })
    }

    /// Labels in wire order.
    pub const fn labels(self) -> [&'static str; 2] {
        self.labels
    }

    /// Interpret an extracted application bit using the commissioned subtype.
    pub fn decode(self, payload: &[u8]) -> Result<&'static str, CodecError> {
        Ok(self.labels[usize::from(crate::decode_switch(payload)?)])
    }

    /// Encode one of the labels returned by this descriptor.
    pub fn encode(self, label: &str) -> Result<[u8; 1], CodecError> {
        if label == self.labels[0] {
            Ok([0])
        } else if label == self.labels[1] {
            Ok([1])
        } else {
            Err(CodecError::OutsideRange)
        }
    }
}

/// KNX two-byte floating measurement (§3.10), expressed in its assigned unit.
/// On devices offering Fahrenheit (9.027) or km/h (9.028), the corresponding
/// Celsius (9.001) or m/s (9.005) point is required and is the commissioning default.
///
/// ```
/// let humidity = knx::Float16Dpt::from_dpt("9.007").unwrap();
/// assert_eq!(humidity.unit(), "%");
/// assert_eq!(humidity.decode(&[0x0c, 0xe2]), Ok(25.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Float16Dpt {
    unit: &'static str,
    minimum: f64,
}

impl Float16Dpt {
    /// Look up an exact identifier and its physical lower bound.
    pub fn from_dpt(id: &str) -> Option<Self> {
        let (unit, minimum) = match id {
            "9.001" => ("°C", -273.0),
            "9.002" => ("K", -671_088.64),
            "9.003" => ("K/h", -671_088.64),
            "9.004" => ("lx", 0.0),
            "9.005" => ("m/s", 0.0),
            "9.006" => ("Pa", 0.0),
            "9.007" => ("%", 0.0),
            "9.008" => ("ppm", 0.0),
            "9.009" => ("m³/h", -671_088.64),
            "9.010" => ("s", -671_088.64),
            "9.011" => ("ms", -671_088.64),
            "9.020" => ("mV", -671_088.64),
            "9.021" => ("mA", -671_088.64),
            "9.022" => ("W/m²", -671_088.64),
            "9.023" => ("K/%", -671_088.64),
            "9.024" => ("kW", -671_088.64),
            "9.025" => ("l/h", -671_088.64),
            "9.026" => ("l/m²", -671_088.64),
            "9.027" => ("°F", -459.6),
            "9.028" => ("km/h", 0.0),
            "9.029" => ("g/m³", 0.0),
            "9.030" => ("µg/m³", 0.0),
            _ => return None,
        };
        Some(Self { unit, minimum })
    }
    /// Physical unit of the encoded number.
    pub const fn unit(self) -> &'static str {
        self.unit
    }
    /// Decode two network-order bytes, rejecting the invalid word and out-of-range measurements.
    pub fn decode(self, payload: &[u8]) -> Result<f64, CodecError> {
        crate::codecs::decode_float16(payload, self.minimum)
    }
    /// Round to the finest fitting exponent, halfway away from zero.
    pub fn encode(self, value: f64) -> Result<[u8; 2], CodecError> {
        crate::codecs::encode_float16(value, self.minimum)
    }
}

/// A finite IEEE binary32 measurement (§3.15). The device supplies the unit for 14.005.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Float32Dpt {
    unit: &'static str,
}

impl Float32Dpt {
    /// Look up an exact identifier. 14.024 follows the edition's printed unit symbol.
    pub fn from_dpt(id: &str) -> Option<Self> {
        let unit = match id {
            "14.000" => "m/s²",
            "14.001" => "rad/s²",
            "14.002" => "J/mol",
            "14.003" => "1/s",
            "14.004" => "mol",
            "14.005" => "application-defined",
            "14.006" => "rad",
            "14.007" => "°",
            "14.008" => "J·s",
            "14.009" => "rad/s",
            "14.010" => "m²",
            "14.011" => "F",
            "14.012" => "C/m²",
            "14.013" => "C/m³",
            "14.014" => "m²/N",
            "14.015" => "S",
            "14.016" => "S/m",
            "14.017" => "kg/m³",
            "14.018" => "C",
            "14.019" => "A",
            "14.020" => "A/m²",
            "14.021" => "C·m",
            "14.022" => "C/m²",
            "14.023" => "V/m",
            "14.024" => "c",
            "14.025" => "C/m²",
            "14.026" => "C/m²",
            "14.027" => "V",
            "14.028" => "V",
            "14.029" => "A·m²",
            "14.030" => "V",
            "14.031" => "J",
            "14.032" => "N",
            "14.033" => "Hz",
            "14.034" => "rad/s",
            "14.035" => "J/K",
            "14.036" => "W",
            "14.037" => "J",
            "14.038" => "Ω",
            "14.039" => "m",
            "14.040" => "J or lm·s",
            "14.041" => "cd/m²",
            "14.042" => "lm",
            "14.043" => "cd",
            "14.044" => "A/m",
            "14.045" => "Wb",
            "14.046" => "T",
            "14.047" => "Wb·m",
            "14.048" => "T",
            "14.049" => "A/m",
            "14.050" => "A",
            "14.051" => "kg",
            "14.052" => "kg/s",
            "14.053" => "N·s",
            "14.054" => "rad",
            "14.055" => "°",
            "14.056" => "W",
            "14.057" => "1",
            "14.058" => "Pa",
            "14.059" => "Ω",
            "14.060" => "Ω",
            "14.061" => "Ω·m",
            "14.062" => "H",
            "14.063" => "sr",
            "14.064" => "W/m²",
            "14.065" => "m/s",
            "14.066" => "Pa",
            "14.067" => "N/m",
            "14.068" => "°C",
            "14.069" => "K",
            "14.070" => "K",
            "14.071" => "J/K",
            "14.072" => "W/(m·K)",
            "14.073" => "V/K",
            "14.074" => "s",
            "14.075" => "N·m",
            "14.076" => "m³",
            "14.077" => "m³/s",
            "14.078" => "N",
            "14.079" => "J",
            "14.080" => "VA",
            "14.1200" => "m³/h",
            "14.1201" => "l/s",
            _ => return None,
        };
        Some(Self { unit })
    }
    /// Unit defined by the commissioned subtype.
    pub const fn unit(self) -> &'static str {
        self.unit
    }
    /// Decode finite network-order binary32, preserving signed zero.
    pub fn decode(self, payload: &[u8]) -> Result<f32, CodecError> {
        crate::power::decode_finite(payload)
    }
    /// Encode a finite measurement without rescaling.
    pub fn encode(self, value: f32) -> Result<[u8; 4], CodecError> {
        crate::power::encode_finite(value)
    }
}

// Each integer assignment uses the complete native range and big-endian byte order.
macro_rules! integer_codec {
    ($decode:ident, $encode:ident, $ty:ty, $width:literal, $id:literal, $unit:literal) => {
        #[doc = concat!("Decode DPT ", $id, " as ", $unit, ".")]
        pub fn $decode(payload: &[u8]) -> Result<$ty, CodecError> {
            Ok(<$ty>::from_be_bytes(bytes(payload)?))
        }
        #[doc = concat!("Encode DPT ", $id, " in ", $unit, ".")]
        pub const fn $encode(value: $ty) -> [u8; $width] {
            value.to_be_bytes()
        }
    };
}
integer_codec!(
    decode_pulse_count_u16,
    encode_pulse_count_u16,
    u16,
    2,
    "7.001",
    "pulses"
);
integer_codec!(
    decode_property_data_type,
    encode_property_data_type,
    u16,
    2,
    "7.010",
    "a numeric property data type identifier; the caller resolves it against its property definitions"
);
integer_codec!(
    decode_length_mm,
    encode_length_mm,
    u16,
    2,
    "7.011",
    "millimetres"
);
integer_codec!(
    decode_parameter_lux,
    encode_parameter_lux,
    u16,
    2,
    "7.013",
    "lux for approved E-Mode parameters"
);
integer_codec!(
    decode_pulse_count_i16,
    encode_pulse_count_i16,
    i16,
    2,
    "8.001",
    "pulses, including the raw maximum for profiles with optional invalid indication"
);
integer_codec!(
    decode_rotation_degrees,
    encode_rotation_degrees,
    i16,
    2,
    "8.011",
    "whole degrees"
);
integer_codec!(
    decode_altitude_m,
    encode_altitude_m,
    i16,
    2,
    "8.012",
    "metres above sea level for profile parameters"
);
integer_codec!(
    decode_pulse_count_u32,
    encode_pulse_count_u32,
    u32,
    4,
    "12.001",
    "pulses"
);
integer_codec!(
    decode_operating_seconds,
    encode_operating_seconds,
    u32,
    4,
    "12.100",
    "cumulative operating seconds"
);
integer_codec!(
    decode_operating_minutes,
    encode_operating_minutes,
    u32,
    4,
    "12.101",
    "cumulative operating minutes"
);
integer_codec!(
    decode_operating_hours,
    encode_operating_hours,
    u32,
    4,
    "12.102",
    "cumulative operating hours"
);
integer_codec!(
    decode_pulse_count_i32,
    encode_pulse_count_i32,
    i32,
    4,
    "13.001",
    "pulses"
);
integer_codec!(
    decode_flow_rate_ticks,
    encode_flow_rate_ticks,
    i32,
    4,
    "13.002",
    "increments of 0.0001 m³/h"
);
integer_codec!(
    decode_operating_delta_seconds,
    encode_operating_delta_seconds,
    i32,
    4,
    "13.100",
    "signed operating seconds"
);

/// Decode DPT 7.012 in mA. Zero denotes absent bus power supply functionality.
pub fn decode_supply_ma(payload: &[u8]) -> Result<Option<NonZeroU16>, CodecError> {
    Ok(NonZeroU16::new(u16::from_be_bytes(bytes(payload)?)))
}
/// Encode DPT 7.012. Use `None` when the device has no bus power supply functionality.
pub fn encode_supply_ma(value: Option<NonZeroU16>) -> [u8; 2] {
    value.map_or(0, NonZeroU16::get).to_be_bytes()
}

/// Decode DPT 8.010 in exact hundredths of a percentage point.
pub fn decode_percent_hundredths(payload: &[u8]) -> Result<i16, CodecError> {
    let ticks = i16::from_be_bytes(bytes(payload)?);
    if ticks == i16::MAX {
        Err(CodecError::InvalidData)
    } else {
        Ok(ticks)
    }
}
/// Encode DPT 8.010 in exact hundredths of a percentage point.
/// The largest valid count is 32766. The next wire code signals invalid data.
pub fn encode_percent_hundredths(ticks: i16) -> Result<[u8; 2], CodecError> {
    if ticks == i16::MAX {
        Err(CodecError::OutsideRange)
    } else {
        Ok(ticks.to_be_bytes())
    }
}

integer_codec!(
    decode_colour_kelvin,
    encode_colour_kelvin,
    u16,
    2,
    "7.600",
    "kelvin"
);

integer_codec!(
    decode_volume_litres,
    encode_volume_litres,
    u32,
    4,
    "12.1200",
    "litres"
);

integer_codec!(
    decode_volume_cubic_metres,
    encode_volume_cubic_metres,
    u32,
    4,
    "12.1201",
    "cubic metres"
);

integer_codec!(
    decode_delta_litres,
    encode_delta_litres,
    i32,
    4,
    "13.1200",
    "litres"
);

integer_codec!(
    decode_delta_cubic_metres,
    encode_delta_cubic_metres,
    i32,
    4,
    "13.1201",
    "cubic metres"
);

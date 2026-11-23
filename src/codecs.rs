//! KNX application values after telegram framing has been removed.

/// A payload or requested value cannot represent the selected datapoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    /// The slice must contain the exact application payload width.
    PayloadLength { expected: usize, actual: usize },
    /// A floating-point input is NaN or infinite.
    NonFinite,
    /// The wire value explicitly denotes invalid data.
    InvalidData,
    /// An in-range interval is not an exact multiple of the commissioned tick.
    InexactInterval { resolution_ms: i64 },
    /// The value falls outside the selected subtype's range.
    OutsideRange,
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::PayloadLength { expected, actual } => {
                return write!(
                    f,
                    "Expected {expected} application bytes, received {actual}."
                );
            }
            Self::NonFinite => "Supply a finite value for the selected DPT.",
            Self::InvalidData => "The source reported invalid data.",
            Self::InexactInterval { resolution_ms } => {
                return write!(f, "Supply an interval in multiples of {resolution_ms} ms.");
            }
            Self::OutsideRange => "Supply a value within the selected DPT range.",
        })
    }
}

impl std::error::Error for CodecError {}

pub(crate) fn bytes<const N: usize>(payload: &[u8]) -> Result<[u8; N], CodecError> {
    payload.try_into().map_err(|_| CodecError::PayloadLength {
        expected: N,
        actual: payload.len(),
    })
}

/// Decode a right-aligned, extracted DPT 1.001 switch bit in a single byte.
pub fn decode_switch(payload: &[u8]) -> Result<bool, CodecError> {
    match bytes(payload)? {
        [0] => Ok(false),
        [1] => Ok(true),
        _ => Err(CodecError::OutsideRange),
    }
}

/// Encode the extracted DPT 1.001 switch bit for subsequent framing.
pub fn encode_switch(on: bool) -> [u8; 1] {
    [u8::from(on)]
}

/// Decode DPT 5.001 scaling as a percentage between 0 and 100.
pub fn decode_scaling_percent(payload: &[u8]) -> Result<f64, CodecError> {
    decode_scaled_byte(payload, 100.0)
}

/// Encode DPT 5.001 to the nearest code, rounding halfway values up.
pub fn encode_scaling_percent(percent: f64) -> Result<[u8; 1], CodecError> {
    encode_scaled_byte(percent, 100.0)
}

/// Decode DPT 5.003 to degrees, including 360° for FF.
pub fn decode_angle_degrees(payload: &[u8]) -> Result<f64, CodecError> {
    decode_scaled_byte(payload, 360.0)
}

/// Encode a finite angle in 0–360° to the nearest byte, rounding half steps up.
pub fn encode_angle_degrees(degrees: f64) -> Result<[u8; 1], CodecError> {
    encode_scaled_byte(degrees, 360.0)
}

fn decode_scaled_byte(payload: &[u8], maximum: f64) -> Result<f64, CodecError> {
    let [raw] = bytes(payload)?;
    Ok(f64::from(raw) * maximum / 255.0)
}

fn encode_scaled_byte(value: f64, maximum: f64) -> Result<[u8; 1], CodecError> {
    if !value.is_finite() {
        return Err(CodecError::NonFinite);
    }
    if !(0.0..=maximum).contains(&value) {
        return Err(CodecError::OutsideRange);
    }
    Ok([(value * 255.0 / maximum).round() as u8])
}

/// Decode DPT 5.004 as whole percent, including values above 100%.
pub fn decode_percent_u8(payload: &[u8]) -> Result<u8, CodecError> {
    let [percent] = bytes(payload)?;
    Ok(percent)
}

/// Encode DPT 5.004 whole percent directly in one application byte.
pub fn encode_percent_u8(percent: u8) -> [u8; 1] {
    [percent]
}

/// Decode the DPT 5.005 integer ratio.
pub fn decode_decimal_factor(payload: &[u8]) -> Result<u8, CodecError> {
    let [factor] = bytes(payload)?;
    Ok(factor)
}

/// Encode a DPT 5.005 integer ratio.
pub const fn encode_decimal_factor(factor: u8) -> [u8; 1] {
    [factor]
}

/// Decode the DPT 5.010 unsigned counter in pulses.
pub fn decode_pulse_count(payload: &[u8]) -> Result<u8, CodecError> {
    let [pulses] = bytes(payload)?;
    Ok(pulses)
}

/// Encode a DPT 5.010 pulse count.
pub const fn encode_pulse_count(pulses: u8) -> [u8; 1] {
    [pulses]
}

/// DPT 5.006 tariff availability and supplier-defined number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tariff(u8);

impl Tariff {
    /// The point reports no available tariff.
    pub const UNAVAILABLE: Self = Self(0);

    /// Select a supplier-defined tariff from 1 through 254.
    pub fn numbered(number: u8) -> Result<Self, CodecError> {
        match number {
            1..=254 => Ok(Self(number)),
            _ => Err(CodecError::OutsideRange),
        }
    }

    /// Return the supplier number, or None when a tariff is unavailable.
    pub fn number(self) -> Option<u8> {
        (self.0 != 0).then_some(self.0)
    }

    /// Decode one tariff byte. Discard the observation if the code is reserved (FF).
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        match bytes(payload)? {
            [0] => Ok(Self::UNAVAILABLE),
            [number] => Self::numbered(number),
        }
    }

    /// Encode the tariff in one application byte.
    pub const fn encode(self) -> [u8; 1] {
        [self.0]
    }
}

/// Decode DPT 6.001 as a signed whole percentage (−128 through 127).
pub fn decode_percent_i8(payload: &[u8]) -> Result<i8, CodecError> {
    Ok(i8::from_be_bytes(bytes(payload)?))
}

/// Encode a DPT 6.001 signed whole percentage.
pub const fn encode_percent_i8(percent: i8) -> [u8; 1] {
    percent.to_be_bytes()
}

/// Decode DPT 6.010 as a signed count in pulses.
pub fn decode_signed_pulse_count(payload: &[u8]) -> Result<i8, CodecError> {
    Ok(i8::from_be_bytes(bytes(payload)?))
}

/// Encode a DPT 6.010 signed pulse count.
pub const fn encode_signed_pulse_count(pulses: i8) -> [u8; 1] {
    pulses.to_be_bytes()
}

/// Decode DPT 9.001 Celsius. The reserved word 0x7fff indicates invalid data.
pub fn decode_temperature_celsius(payload: &[u8]) -> Result<f64, CodecError> {
    decode_float16(payload, -273.0)
}

pub(crate) fn decode_float16(payload: &[u8], minimum: f64) -> Result<f64, CodecError> {
    let word = u16::from_be_bytes(bytes(payload)?);
    if word == 0x7fff {
        return Err(CodecError::InvalidData);
    }
    let exponent = (word >> 11) & 15;
    let magnitude = i32::from(word & 0x07ff);
    let mantissa = if word & 0x8000 == 0 {
        magnitude
    } else {
        magnitude - 2048
    };
    let value = f64::from(mantissa << exponent) / 100.0;
    if value < minimum {
        return Err(CodecError::OutsideRange);
    }
    Ok(value)
}

/// Encode DPT 9.001 Celsius at the finest available precision. Round halfway
/// mantissas away from zero. The accepted input range is −273 to 670433.28 °C.
pub fn encode_temperature_celsius(celsius: f64) -> Result<[u8; 2], CodecError> {
    encode_float16(celsius, -273.0)
}

pub(crate) fn encode_float16(value: f64, minimum: f64) -> Result<[u8; 2], CodecError> {
    if !value.is_finite() {
        return Err(CodecError::NonFinite);
    }
    if !(minimum..=670_433.28).contains(&value) {
        return Err(CodecError::OutsideRange);
    }
    let mut scaled = value * 100.0;
    let mut exponent = 0u16;
    while !(-2048.0..=2047.0).contains(&scaled.round()) {
        scaled /= 2.0;
        exponent += 1;
    }
    let mut mantissa = scaled.round() as i16;
    // Rounding at a subtype boundary must not produce a value its decoder rejects.
    if f64::from(i32::from(mantissa) << exponent) / 100.0 < minimum {
        mantissa += 1;
    }
    let sign = if mantissa < 0 { 0x8000 } else { 0 };
    Ok((sign | (exponent << 11) | ((mantissa as u16) & 0x07ff)).to_be_bytes())
}

/// Decode DPT 13.010 active energy in signed watt-hours.
pub fn decode_energy_wh(payload: &[u8]) -> Result<i32, CodecError> {
    crate::Energy32::ActiveWh.decode(payload)
}

/// Encode DPT 13.010 active energy in signed watt-hours.
pub fn encode_energy_wh(wh: i32) -> [u8; 4] {
    crate::Energy32::ActiveWh.encode(wh)
}

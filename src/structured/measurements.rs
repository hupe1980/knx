use super::{LteCommand, LteDpt, LteValue, Representation};
use crate::{CodecError, Float32Dpt, codecs::bytes, text_time::*};

/// DPT 229.001 ValInfField. Counts have the reported unit times 10 to the decimal exponent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeterScale(u8);
impl MeterScale {
    pub fn from_code(code: u8) -> Result<Self, CodecError> {
        match code {
            0x00..=0x1f
            | 0x28..=0x57
            | 0x6e
            | 0x80..=0x81
            | 0x88..=0x89
            | 0xa8..=0xa9
            | 0xb0..=0xb1
            | 0xba => Ok(Self(code)),
            _ => Err(CodecError::InvalidData),
        }
    }
    pub const fn code(self) -> u8 {
        self.0
    }
    pub const fn unit_and_exponent(self) -> (&'static str, i8) {
        let n = (self.0 & 7) as i8;
        match self.0 {
            0..=7 => ("Wh", n - 3),
            8..=15 => ("J", n),
            16..=23 => ("m³", n - 6),
            24..=31 => ("kg", n - 3),
            0x28..=0x2f => ("W", n - 3),
            0x30..=0x37 => ("J/h", n),
            0x38..=0x3f => ("m³/h", n - 6),
            0x40..=0x47 => ("m³/min", n - 7),
            0x48..=0x4f => ("m³/s", n - 9),
            0x50..=0x57 => ("kg/h", n - 3),
            0x6e => ("HCA unit", 0),
            0x80..=0x81 => ("Wh", n + 5),
            0x88..=0x89 => ("J", n + 8),
            0xa8..=0xa9 => ("W", n + 5),
            0xb0..=0xb1 => ("J/h", n + 8),
            _ => ("count", 0),
        }
    }
}
const METER: LteDpt = LteDpt {
    repr: Representation::V32,
    tick: "count",
};
/// DPT 229.001 metering report or command. The counted medium comes from commissioning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeteringValue<T> {
    pub value: T,
    pub scale: MeterScale,
}
impl MeteringValue<LteValue> {
    pub fn decode_report(payload: &[u8]) -> Result<Self, CodecError> {
        let [a, b, c, d, vif, z] = bytes(payload)?;
        Ok(Self {
            value: METER.decode_report(&[a, b, c, d, z])?,
            scale: MeterScale::from_code(vif)?,
        })
    }
    pub fn encode_report(self) -> Result<[u8; 6], CodecError> {
        let mut packed = [0; 5];
        METER.encode_report(self.value, &mut packed)?;
        let [a, b, c, d, z] = packed;
        Ok([a, b, c, d, self.scale.code(), z])
    }
}
impl MeteringValue<LteCommand> {
    pub fn decode_command(payload: &[u8]) -> Result<Self, CodecError> {
        let [a, b, c, d, vif, z] = bytes(payload)?;
        Ok(Self {
            value: METER.decode_command(&[a, b, c, d, z])?,
            scale: MeterScale::from_code(vif)?,
        })
    }
    pub fn encode_command(self) -> Result<[u8; 6], CodecError> {
        let mut packed = [0; 5];
        METER.encode_command(self.value, &mut packed)?;
        let [a, b, c, d, z] = packed;
        Ok([a, b, c, d, self.scale.code(), z])
    }
}

/// DPT 225.003 proposed next tariff. Zero delay means the time is unknown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NextTariff {
    pub delay_minutes: Option<std::num::NonZeroU16>,
    pub tariff: u8,
}
impl NextTariff {
    pub fn decode(payload: &[u8]) -> Result<Option<Self>, CodecError> {
        let [h, l, tariff] = bytes(payload)?;
        if tariff == 255 {
            return Err(CodecError::InvalidData);
        }
        let delay_minutes = std::num::NonZeroU16::new(u16::from_be_bytes([h, l]));
        Ok((delay_minutes.is_some() || tariff != 0).then_some(Self {
            delay_minutes,
            tariff,
        }))
    }
    pub fn encode(value: Option<Self>) -> Result<[u8; 3], CodecError> {
        let Some(value) = value else {
            return Ok([0; 3]);
        };
        if value.tariff == 255 || value.tariff == 0 && value.delay_minutes.is_none() {
            return Err(CodecError::OutsideRange);
        }
        let [h, l] = value.delay_minutes.map_or(0, |v| v.get()).to_be_bytes();
        Ok([h, l, value.tariff])
    }
}

/// DPT 221.001 manufacturer code and manufacturer-assigned serial number.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerialNumber {
    pub manufacturer: u16,
    pub number: u32,
}
impl SerialNumber {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [a, b, c, d, e, f] = bytes(payload)?;
        Ok(Self {
            manufacturer: u16::from_be_bytes([a, b]),
            number: u32::from_be_bytes([c, d, e, f]),
        })
    }
    pub const fn encode(self) -> [u8; 6] {
        let [a, b] = self.manufacturer.to_be_bytes();
        let [c, d, e, f] = self.number.to_be_bytes();
        [a, b, c, d, e, f]
    }
}

/// DPT 239.001. Scaling ranges from 0 to 255 across 0 to 100 percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlaggedScaling {
    pub scaling: u8,
    pub channel_active: bool,
}
impl FlaggedScaling {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [scaling, flag] = bytes(payload)?;
        Ok(Self {
            scaling,
            channel_active: crate::decode_switch(&[flag])?,
        })
    }
    pub const fn encode(self) -> [u8; 2] {
        [self.scaling, self.channel_active as u8]
    }
}

/// DPT 255.001 carries finite longitude first, then latitude, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub longitude_degrees: f32,
    pub latitude_degrees: f32,
}
impl Location {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let b = bytes::<8>(payload)?;
        Ok(Self {
            longitude_degrees: crate::power::decode_finite(&b[..4])?,
            latitude_degrees: crate::power::decode_finite(&b[4..])?,
        })
    }
    pub fn encode(self) -> Result<[u8; 8], CodecError> {
        let mut b = [0; 8];
        b[..4].copy_from_slice(&crate::power::encode_finite(self.longitude_degrees)?);
        b[4..].copy_from_slice(&crate::power::encode_finite(self.latitude_degrees)?);
        Ok(b)
    }
}

/// DPT 256.001. Both boundaries retain all KNX validity flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTimePeriod {
    pub start: DateTime,
    pub stop: DateTime,
}
impl DateTimePeriod {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let b = bytes::<16>(payload)?;
        Ok(Self {
            start: decode_date_time(&b[..8])?,
            stop: decode_date_time(&b[8..])?,
        })
    }
    pub fn encode(self) -> Result<[u8; 16], CodecError> {
        let mut b = [0; 16];
        b[..8].copy_from_slice(&encode_date_time(self.start)?);
        b[8..].copy_from_slice(&encode_date_time(self.stop)?);
        Ok(b)
    }
}

/// A value accompanied by wire date/time, including partially specified dates or clock faults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dated<T> {
    pub at: DateTime,
    pub value: T,
}

/// DPT 265.001/.005/.009/.011/.012 use the assigned 1.xxx function labels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatedBinary(crate::BinaryDpt);
impl DatedBinary {
    pub fn from_dpt(id: &str) -> Option<Self> {
        Some(Self(crate::BinaryDpt::from_dpt(match id {
            "265.001" => "1.001",
            "265.005" => "1.005",
            "265.009" => "1.009",
            "265.011" => "1.011",
            "265.012" => "1.012",
            _ => return None,
        })?))
    }
    pub fn decode(self, payload: &[u8]) -> Result<Dated<&'static str>, CodecError> {
        let b = bytes::<9>(payload)?;
        Ok(Dated {
            at: decode_date_time(&b[..8])?,
            value: self.0.decode(&b[8..])?,
        })
    }
    pub fn encode(self, value: Dated<&str>) -> Result<[u8; 9], CodecError> {
        let mut b = [0; 9];
        b[..8].copy_from_slice(&encode_date_time(value.at)?);
        b[8] = self.0.encode(value.value)?[0];
        Ok(b)
    }
}

/// DPT 266.027/.056/.080 carry electrical potential, active power or apparent power.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatedMeasurement(Float32Dpt);
impl DatedMeasurement {
    pub fn from_dpt(id: &str) -> Option<Self> {
        Some(Self(Float32Dpt::from_dpt(match id {
            "266.027" => "14.027",
            "266.056" => "14.056",
            "266.080" => "14.080",
            _ => return None,
        })?))
    }
    pub const fn unit(self) -> &'static str {
        self.0.unit()
    }
    pub fn decode(self, payload: &[u8]) -> Result<Dated<f32>, CodecError> {
        let b = bytes::<12>(payload)?;
        Ok(Dated {
            at: decode_date_time(&b[..8])?,
            value: self.0.decode(&b[8..])?,
        })
    }
    pub fn encode(self, value: Dated<f32>) -> Result<[u8; 12], CodecError> {
        let mut b = [0; 12];
        b[..8].copy_from_slice(&encode_date_time(value.at)?);
        b[8..].copy_from_slice(&self.0.encode(value.value)?);
        Ok(b)
    }
}

/// DPT 267.001, borrowed UTF-8 following a KNX timestamp.
pub fn decode_dated_text(payload: &[u8]) -> Result<Dated<&str>, CodecError> {
    let (at, data) = payload
        .split_at_checked(8)
        .ok_or(CodecError::PayloadLength {
            expected: 9,
            actual: payload.len(),
        })?;
    Ok(Dated {
        at: decode_date_time(at)?,
        value: decode_utf8(data)?,
    })
}
pub fn encode_dated_text<'a>(
    value: Dated<&str>,
    dst: &'a mut [u8],
) -> Result<&'a [u8], CodecError> {
    let at = encode_date_time(value.at)?;
    let len = dst.len();
    let (head, tail) = dst
        .split_at_mut_checked(8)
        .ok_or(CodecError::PayloadLength {
            expected: 9,
            actual: len,
        })?;
    let used = encode_utf8(value.value, tail)?.len();
    head.copy_from_slice(&at);
    Ok(&dst[..8 + used])
}

/// Lighting speed (225.001) or scaling step-time parameter (225.002).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalingTimingDpt {
    Speed,
    StepTime,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScalingTiming {
    pub milliseconds: u32,
    pub scaling_code: std::num::NonZeroU8,
}
impl ScalingTimingDpt {
    fn tick(self) -> u32 {
        match self {
            Self::Speed => 100,
            Self::StepTime => 1,
        }
    }
    pub fn decode(self, payload: &[u8]) -> Result<ScalingTiming, CodecError> {
        let [h, l, code] = bytes(payload)?;
        let ticks = u16::from_be_bytes([h, l]);
        if ticks == 0 {
            return Err(CodecError::InvalidData);
        }
        Ok(ScalingTiming {
            milliseconds: u32::from(ticks) * self.tick(),
            scaling_code: std::num::NonZeroU8::new(code).ok_or(CodecError::InvalidData)?,
        })
    }
    pub fn encode(self, value: ScalingTiming) -> Result<[u8; 3], CodecError> {
        let tick = self.tick();
        if value.milliseconds < tick || value.milliseconds > u32::from(u16::MAX) * tick {
            return Err(CodecError::OutsideRange);
        }
        if !value.milliseconds.is_multiple_of(tick) {
            return Err(CodecError::InexactInterval {
                resolution_ms: i64::from(tick),
            });
        }
        let [h, l] = ((value.milliseconds / tick) as u16).to_be_bytes();
        Ok([h, l, value.scaling_code.get()])
    }
}
/// DPT 236.001 priority activation and mode. Level seven has highest priority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrioritisedMode {
    Activate {
        priority: u8,
        mode: u8,
    },
    /// Deactivation masks the received mode. The encoder clears the unused nibble.
    Deactivate {
        priority: u8,
    },
}
impl PrioritisedMode {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [b] = bytes(payload)?;
        let priority = (b >> 4) & 7;
        Ok(if b & 0x80 == 0 {
            Self::Activate {
                priority,
                mode: b & 15,
            }
        } else {
            Self::Deactivate { priority }
        })
    }

    pub fn encode(self) -> Result<[u8; 1], CodecError> {
        match self {
            Self::Activate {
                priority: p @ 0..=7,
                mode: m @ 0..=15,
            } => Ok([p << 4 | m]),
            Self::Deactivate {
                priority: p @ 0..=7,
            } => Ok([0x80 | p << 4]),
            _ => Err(CodecError::OutsideRange),
        }
    }
}

/// DPT 15.000 reports a six-digit access code and the controlling device's decision.
/// The digits are deliberately omitted from Debug output.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AccessData {
    pub digits: [u8; 6],
    pub read_error: bool,
    pub accepted: bool,
    pub right_to_left: bool,
    pub encrypted: bool,
    pub index: u8,
}
impl std::fmt::Debug for AccessData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessData")
            .field("read_error", &self.read_error)
            .field("accepted", &self.accepted)
            .finish_non_exhaustive()
    }
}
impl AccessData {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [a, b, c, z] = bytes(payload)?;
        let digits = [a >> 4, a & 15, b >> 4, b & 15, c >> 4, c & 15];
        if digits.iter().any(|d| *d > 9) {
            return Err(CodecError::InvalidData);
        }
        Ok(Self {
            digits,
            read_error: z & 128 != 0,
            accepted: z & 64 != 0,
            right_to_left: z & 32 != 0,
            encrypted: z & 16 != 0,
            index: z & 15,
        })
    }
    pub fn encode(self) -> Result<[u8; 4], CodecError> {
        if self.index > 15 || self.digits.iter().any(|d| *d > 9) {
            return Err(CodecError::OutsideRange);
        }
        let [a, b, c, d, e, f] = self.digits;
        Ok([
            a << 4 | b,
            c << 4 | d,
            e << 4 | f,
            u8::from(self.read_error) << 7
                | u8::from(self.accepted) << 6
                | u8::from(self.right_to_left) << 5
                | u8::from(self.encrypted) << 4
                | self.index,
        ])
    }
}
/// DPT 219.001 alarm source capabilities, alarm state and source-defined error class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlarmInfo {
    pub log_number: u8,
    pub priority: super::ParameterPriority,
    pub area: super::ApplicationArea,
    pub error_class: u8,
    pub can_acknowledge: bool,
    pub has_timestamp: bool,
    pub has_alarm_text: bool,
    pub has_error_code: bool,
    pub in_alarm: bool,
    pub unacknowledged: bool,
    pub locked: bool,
}
impl AlarmInfo {
    fn validate(self) -> Result<Self, CodecError> {
        use super::ApplicationArea::*;
        let max = match self.area {
            NoFault => 0,
            System => 5,
            GeneralHvac
            | HotWaterHeating
            | DirectElectricalHeating
            | TerminalUnits
            | VentilationAirConditioning => 4,
            _ => u8::MAX,
        };
        if self.error_class > max || !self.can_acknowledge && self.unacknowledged {
            return Err(CodecError::InvalidData);
        }
        Ok(self)
    }
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [log_number, priority, area, error_class, cap, state] = bytes(payload)?;
        if cap & 0xf0 != 0 || state & 0xf8 != 0 {
            return Err(CodecError::InvalidData);
        }
        Self {
            log_number,
            priority: super::ParameterPriority::decode(&[priority])?,
            area: super::ApplicationArea::decode(&[area])?,
            error_class,
            can_acknowledge: cap & 1 != 0,
            has_timestamp: cap & 2 != 0,
            has_alarm_text: cap & 4 != 0,
            has_error_code: cap & 8 != 0,
            in_alarm: state & 1 != 0,
            unacknowledged: state & 2 != 0,
            locked: state & 4 != 0,
        }
        .validate()
    }
    pub fn encode(self) -> Result<[u8; 6], CodecError> {
        self.validate()?;
        Ok([
            self.log_number,
            self.priority as u8,
            self.area as u8,
            self.error_class,
            u8::from(self.can_acknowledge)
                | u8::from(self.has_timestamp) << 1
                | u8::from(self.has_alarm_text) << 2
                | u8::from(self.has_error_code) << 3,
            u8::from(self.in_alarm)
                | u8::from(self.unacknowledged) << 1
                | u8::from(self.locked) << 2,
        ])
    }
}

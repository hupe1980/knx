//! KNX common compound datapoints for reports, parameter writes and actuator feedback.
//!
//! The final octet of an LTE compound is a status for responses and info reports,
//! and a command for property writes. Select the decoder from the application service.
//! Standard group objects use their separately commissioned DPT.
//!
//! ```
//! use knx::structured::{LteDpt, LteValue};
//! let power = LteDpt::from_dpt("203.014").unwrap();
//! assert_eq!(power.tick_unit(), "kW");
//! assert_eq!(power.decode_report(&[0, 12, 4])?, LteValue::Overridden { ticks: 12, fault: false });
//! # Ok::<(), knx::CodecError>(())
//! ```
use crate::{CodecError, SceneNumber, codecs::bytes};

/// Meaning of the elementary field in an LTE report. Counts are in the selected DPT's native ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LteValue {
    Measured(i64),
    Overridden { ticks: i64, fault: bool },
    OutOfService,
    Fault(FaultReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FaultReason {
    Unspecified = 0,
    OpenCircuit = 1,
    ShortCircuit = 2,
}

/// Commands have their own Z8 interpretation. A device profile determines which it accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LteCommand {
    Write(i64),
    Override(i64),
    Release,
    SetOutOfService,
    ResetOutOfService(i64),
    AcknowledgeAlarm,
    SetToDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Representation {
    Codes(&'static [&'static str]),
    U16Sun,
    V16Temperature,
    U8,
    V8Percent,
    U16,
    V16,
    V32,
}

/// Common and HVAC compound subtypes in §§3.31–3.41, 3.43 and 4.8–4.11. Exact counts avoid rounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LteDpt {
    repr: Representation,
    tick: &'static str,
}

impl LteDpt {
    pub fn from_dpt(id: &str) -> Option<Self> {
        use Representation::*;
        let (repr, tick) = match id {
            "200.100" => (Codes(&["cooling", "heating"]), "code"),
            "200.101" => (Codes(&["low", "high"]), "code"),
            "201.100" => (
                Codes(&[
                    "auto",
                    "comfort",
                    "standby",
                    "economy",
                    "building-protection",
                ]),
                "code",
            ),
            "201.102" => (
                Codes(&[
                    "auto",
                    "legionella-protection",
                    "normal",
                    "reduced",
                    "off-frost-protection",
                ]),
                "code",
            ),
            "201.104" => (
                Codes(&[
                    "auto",
                    "heat",
                    "morning-warmup",
                    "cool",
                    "night-purge",
                    "precool",
                    "off",
                    "test",
                    "emergency-heat",
                    "fan-only",
                    "free-cool",
                    "ice",
                    "maximum-heating",
                    "economic-heat-cool",
                    "dehumidification",
                    "calibration",
                    "emergency-cool",
                    "emergency-steam",
                    "",
                    "",
                    "no-demand",
                ]),
                "code",
            ),
            "201.105" => (Codes(&["disabled", "stage-a", "stage-b", "both"]), "code"),
            "201.107" => (Codes(&["in-use", "unused", "protection"]), "code"),
            "201.108" => (Codes(&["occupied", "standby", "unoccupied"]), "code"),
            "201.109" => (
                Codes(&[
                    "normal",
                    "emergency-pressure",
                    "emergency-depressure",
                    "emergency-purge",
                    "emergency-shutdown",
                    "emergency-fire",
                ]),
                "code",
            ),
            "203.100" => (U16, "ppm"),
            "203.101" => (U16, "0.01 m/s"),
            "203.102" => (U16Sun, "0.05 W/m²"),
            "203.104" => (U16, "m³/h"),
            "205.100" => (V16Temperature, "0.02 °C"),
            "205.101" => (V16Temperature, "0.02 K"),
            "205.102" => (V16, "m³/h"),
            "205.103" => (V16, "ppm"),
            "202.001" => (U8, "%"),
            "202.002" => (U8, "count"),
            "203.002" => (U16, "ms"),
            "203.003" => (U16, "10 ms"),
            "203.004" => (U16, "100 ms"),
            "203.005" => (U16, "s"),
            "203.006" => (U16, "min"),
            "203.007" => (U16, "h"),
            "203.011" => (U16, "0.01 l/h"),
            "203.012" => (U16, "count"),
            "203.013" => (U16, "0.01 µA"),
            "203.014" => (U16, "kW"),
            "203.015" => (U16, "0.05 mbar"),
            "203.017" => (U16, "0.01 %"),
            "204.001" => (V8Percent, "%"),
            "205.002" => (V16, "ms"),
            "205.003" => (V16, "10 ms"),
            "205.004" => (V16, "100 ms"),
            "205.005" => (V16, "s"),
            "205.006" => (V16, "min"),
            "205.007" => (V16, "h"),
            "205.017" => (V16, "0.01 %"),
            "218.001" => (V32, "l"),
            "218.002" => (V32, "0.0001 m³/h"),
            _ => return None,
        };
        Some(Self { repr, tick })
    }
    pub const fn tick_unit(self) -> &'static str {
        self.tick
    }
    /// Assigned meaning of a binary or enumerated value. Numeric quantities and
    /// reserved codes have no label.
    pub fn label(self, code: i64) -> Option<&'static str> {
        let Representation::Codes(labels) = self.repr else {
            return None;
        };
        labels
            .get(usize::try_from(code).ok()?)
            .copied()
            .filter(|label| !label.is_empty())
    }
    /// Exact compound payload width including Z8.
    pub const fn payload_len(self) -> usize {
        use Representation::*;
        match self.repr {
            Codes(_) | U8 | V8Percent => 2,
            U16Sun | V16Temperature | U16 | V16 => 3,
            V32 => 5,
        }
    }

    fn fields(self, payload: &[u8]) -> Result<(i64, u8), CodecError> {
        if payload.len() != self.payload_len() {
            return Err(CodecError::PayloadLength {
                expected: self.payload_len(),
                actual: payload.len(),
            });
        }
        use Representation::*;
        let main = match self.repr {
            Codes(_) | U8 => i64::from(payload[0]),
            V8Percent => i64::from(payload[0] as i8),
            U16Sun | U16 => i64::from(u16::from_be_bytes([payload[0], payload[1]])),
            V16Temperature | V16 => i64::from(i16::from_be_bytes([payload[0], payload[1]])),
            V32 => i64::from(i32::from_be_bytes([
                payload[0], payload[1], payload[2], payload[3],
            ])),
        };
        Ok((main, payload[payload.len() - 1]))
    }

    fn ticks(self, value: i64) -> Result<i64, CodecError> {
        use Representation::*;
        let valid = match self.repr {
            Codes(_) => self.label(value).is_some(),
            U16Sun => (0..=28_000).contains(&value),
            V16Temperature => (-13_650..=32_767).contains(&value),
            U8 => u8::try_from(value).is_ok(),
            V8Percent => (-100..=100).contains(&value),
            U16 => u16::try_from(value).is_ok(),
            V16 => i16::try_from(value).is_ok(),
            V32 => i32::try_from(value).is_ok(),
        };
        if valid {
            Ok(value)
        } else {
            Err(CodecError::OutsideRange)
        }
    }

    fn put(self, ticks: i64, z8: u8, dst: &mut [u8]) -> Result<(), CodecError> {
        let len = self.payload_len();
        if dst.len() != len {
            return Err(CodecError::PayloadLength {
                expected: len,
                actual: dst.len(),
            });
        }
        let value = ticks.to_be_bytes();
        dst[..len - 1].copy_from_slice(&value[9 - len..]);
        dst[len - 1] = z8;
        Ok(())
    }

    /// Decode a property response or info report. These common quantities do not carry alarm flags.
    pub fn decode_report(self, payload: &[u8]) -> Result<LteValue, CodecError> {
        let (ticks, status) = self.fields(payload)?;
        Ok(match status {
            0 => LteValue::Measured(self.ticks(ticks)?),
            1 => LteValue::OutOfService,
            2 => LteValue::Fault(match ticks {
                0 => FaultReason::Unspecified,
                1 => FaultReason::OpenCircuit,
                2 => FaultReason::ShortCircuit,
                _ => return Err(CodecError::InvalidData),
            }),
            4 | 6 => LteValue::Overridden {
                ticks: self.ticks(ticks)?,
                fault: status == 6,
            },
            _ => return Err(CodecError::InvalidData),
        })
    }
    /// Write a report into an exact-width buffer. Void fields are zeroed.
    pub fn encode_report(self, value: LteValue, dst: &mut [u8]) -> Result<(), CodecError> {
        let (ticks, status) = match value {
            LteValue::Measured(t) => (self.ticks(t)?, 0),
            LteValue::OutOfService => (0, 1),
            LteValue::Fault(r) => (r as i64, 2),
            LteValue::Overridden { ticks, fault } => {
                (self.ticks(ticks)?, if fault { 6 } else { 4 })
            }
        };
        self.put(ticks, status, dst)
    }
    /// Decode the property write service. Commands with unused main values discard those bytes.
    pub fn decode_command(self, payload: &[u8]) -> Result<LteCommand, CodecError> {
        let (ticks, command) = self.fields(payload)?;
        Ok(match command {
            0 => LteCommand::Write(self.ticks(ticks)?),
            1 => LteCommand::Override(self.ticks(ticks)?),
            2 => LteCommand::Release,
            3 => LteCommand::SetOutOfService,
            4 => LteCommand::ResetOutOfService(self.ticks(ticks)?),
            5 => LteCommand::AcknowledgeAlarm,
            6 => LteCommand::SetToDefault,
            _ => return Err(CodecError::InvalidData),
        })
    }
    pub fn encode_command(self, value: LteCommand, dst: &mut [u8]) -> Result<(), CodecError> {
        let (ticks, command) = match value {
            LteCommand::Write(t) => (self.ticks(t)?, 0),
            LteCommand::Override(t) => (self.ticks(t)?, 1),
            LteCommand::Release => (0, 2),
            LteCommand::SetOutOfService => (0, 3),
            LteCommand::ResetOutOfService(t) => (self.ticks(t)?, 4),
            LteCommand::AcknowledgeAlarm => (0, 5),
            LteCommand::SetToDefault => (0, 6),
        };
        self.put(ticks, command, dst)
    }
}

/// DPT 21.001 independent status datapoint. The main value arrives separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneralStatus {
    pub out_of_service: bool,
    pub fault: bool,
    pub overridden: bool,
    pub in_alarm: bool,
    pub alarm_unacknowledged: bool,
}
impl GeneralStatus {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [b] = bytes(payload)?;
        if b & 0xe0 != 0 || b & 1 != 0 && b & 6 != 0 {
            return Err(CodecError::InvalidData);
        }
        Ok(Self {
            out_of_service: b & 1 != 0,
            fault: b & 2 != 0,
            overridden: b & 4 != 0,
            in_alarm: b & 8 != 0,
            alarm_unacknowledged: b & 16 != 0,
        })
    }
    pub fn encode(self) -> Result<[u8; 1], CodecError> {
        if self.out_of_service && (self.fault || self.overridden) {
            return Err(CodecError::InvalidData);
        }
        Ok([u8::from(self.out_of_service)
            | u8::from(self.fault) << 1
            | u8::from(self.overridden) << 2
            | u8::from(self.in_alarm) << 3
            | u8::from(self.alarm_unacknowledged) << 4])
    }
}

/// DPT 21.002, PID_DEVICE_CONTROL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceControl {
    pub user_stopped: bool,
    pub own_address_received: bool,
    pub verify_mode: bool,
}
impl DeviceControl {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [b] = bytes(payload)?;
        if b & 0xf8 != 0 {
            return Err(CodecError::InvalidData);
        }
        Ok(Self {
            user_stopped: b & 1 != 0,
            own_address_received: b & 2 != 0,
            verify_mode: b & 4 != 0,
        })
    }
    pub const fn encode(self) -> [u8; 1] {
        [self.user_stopped as u8
            | (self.own_address_received as u8) << 1
            | (self.verify_mode as u8) << 2]
    }
}

/// DPT 26.001 scene status. Scene numbers follow ETS's 1–64 convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneInfo {
    pub scene: SceneNumber,
    pub active: bool,
}
impl SceneInfo {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [b] = bytes(payload)?;
        if b & 0x80 != 0 {
            return Err(CodecError::InvalidData);
        }
        Ok(Self {
            scene: SceneNumber::new((b & 63) + 1)?,
            active: b & 64 == 0,
        })
    }
    pub fn encode(self) -> [u8; 1] {
        [(self.scene.number() - 1) | u8::from(!self.active) << 6]
    }
}

/// DPT 238.001 includes a storage permission in addition to the scene status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneConfig {
    pub info: SceneInfo,
    pub storage_enabled: bool,
}
impl SceneConfig {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [b] = bytes(payload)?;
        Ok(Self {
            info: SceneInfo::decode(&[b & 127])?,
            storage_enabled: b & 128 == 0,
        })
    }
    pub fn encode(self) -> [u8; 1] {
        [self.info.encode()[0] | u8::from(!self.storage_enabled) << 7]
    }
}

/// DPT 27.001 reports up to sixteen independently valid on/off states. Index zero is output 1.
pub fn decode_output_states(payload: &[u8]) -> Result<[Option<bool>; 16], CodecError> {
    let [m1, m0, s1, s0] = bytes(payload)?;
    let mask = u16::from_be_bytes([m1, m0]);
    let states = u16::from_be_bytes([s1, s0]);
    Ok(std::array::from_fn(|i| {
        (mask & (1 << i) != 0).then_some(states & (1 << i) != 0)
    }))
}
pub fn encode_output_states(states: [Option<bool>; 16]) -> [u8; 4] {
    let mut valid = 0u16;
    let mut on = 0u16;
    for (i, state) in states.into_iter().enumerate() {
        if let Some(value) = state {
            valid |= 1 << i;
            on |= u16::from(value) << i;
        }
    }
    let [m1, m0] = valid.to_be_bytes();
    let [s1, s0] = on.to_be_bytes();
    [m1, m0, s1, s0]
}

/// DPT 217.001 compatibility number, feature version and revision. All ones means undefined.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Version {
    pub magic: u8,
    pub version: u8,
    pub revision: u8,
}
impl Version {
    pub fn decode(payload: &[u8]) -> Result<Option<Self>, CodecError> {
        let word = u16::from_be_bytes(bytes(payload)?);
        Ok((word != u16::MAX).then_some(Self {
            magic: (word >> 11) as u8,
            version: ((word >> 6) & 31) as u8,
            revision: (word & 63) as u8,
        }))
    }
    pub fn encode(value: Option<Self>) -> Result<[u8; 2], CodecError> {
        let Some(Self {
            magic,
            version,
            revision,
        }) = value
        else {
            return Ok([255, 255]);
        };
        if magic > 31 || version > 31 || revision > 63 || (magic, version, revision) == (31, 31, 63)
        {
            return Err(CodecError::OutsideRange);
        }
        Ok((u16::from(magic) << 11 | u16::from(version) << 6 | u16::from(revision)).to_be_bytes())
    }
}

mod measurements;
pub use measurements::*;
mod modes;
pub use modes::*;
mod locale;
pub use locale::*;

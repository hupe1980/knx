//! Tariff energy registers and room operating modes for commissioned building points.

use crate::codecs::bytes;
use crate::{CodecError, Tariff};

/// DPT 235.001. Each field has an independent validity bit.
/// A valid tariff can itself report that no tariff is available (code zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TariffEnergy {
    pub watt_hours: Option<i32>,
    pub tariff: Option<Tariff>,
}

impl TariffEnergy {
    /// Decode signed Wh and the associated tariff. Ignore bytes of invalid fields.
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [a, b, c, d, tariff, flags] = bytes(payload)?;
        if flags & 0xfc != 0 {
            return Err(CodecError::InvalidData);
        }
        let tariff = if flags & 1 == 0 {
            Some(Tariff::decode(&[tariff])?)
        } else {
            None
        };
        Ok(Self {
            watt_hours: (flags & 2 == 0).then_some(i32::from_be_bytes([a, b, c, d])),
            tariff,
        })
    }

    /// Set missing-field flags and clear unused bits and field bytes.
    pub fn encode(self) -> [u8; 6] {
        let [a, b, c, d] = self.watt_hours.unwrap_or_default().to_be_bytes();
        let flags = u8::from(self.tariff.is_none()) | (u8::from(self.watt_hours.is_none()) << 1);
        [
            a,
            b,
            c,
            d,
            self.tariff.map_or(0, |value| value.encode()[0]),
            flags,
        ]
    }
}

/// DPT 20.102 command for the room controller operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HvacMode {
    Auto = 0,
    Comfort = 1,
    Standby = 2,
    Economy = 3,
    BuildingProtection = 4,
}

impl HvacMode {
    /// Decode the commissioned mode and reject reserved codes.
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        match bytes(payload)? {
            [0] => Ok(Self::Auto),
            [1] => Ok(Self::Comfort),
            [2] => Ok(Self::Standby),
            [3] => Ok(Self::Economy),
            [4] => Ok(Self::BuildingProtection),
            _ => Err(CodecError::OutsideRange),
        }
    }

    pub const fn encode(self) -> [u8; 1] {
        [self as u8]
    }
}

/// DPT 269.1200. Local timestamp quality and register validity are independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatedTariffEnergy {
    pub timestamp: crate::text_time::DateTime,
    pub register: TariffEnergy,
}

impl DatedTariffEnergy {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let raw: [u8; 14] = bytes(payload)?;
        Ok(Self {
            timestamp: crate::text_time::decode_clq_date_time(&raw[..8])?,
            register: TariffEnergy::decode(&raw[8..])?,
        })
    }

    pub fn encode(self) -> Result<[u8; 14], CodecError> {
        let mut raw = [0; 14];
        raw[..8].copy_from_slice(&crate::text_time::encode_clq_date_time(self.timestamp)?);
        raw[8..].copy_from_slice(&self.register.encode());
        Ok(raw)
    }
}

/// DPT 207.600 status for a lighting actuator. Choose the commissioned actuator kind
/// when decoding to enforce the binary level and dimming constraints of a switch.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LightingActuator {
    Switch,
    Dimmer,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LightingStatus {
    /// DPT 5.001 scaling code (0 = 0%, 255 = 100%). None when invalid.
    pub level: Option<u8>,
    pub locked: bool,
    pub forced: bool,
    pub night_mode: bool,
    pub staircase: bool,
    pub dimming: bool,
    pub local_override: bool,
    pub failure: bool,
}

impl LightingStatus {
    pub fn decode(actuator: LightingActuator, payload: &[u8]) -> Result<Self, CodecError> {
        let [level, flags] = bytes(payload)?;
        let status = Self {
            level: (flags & 1 != 0).then_some(level),
            locked: flags & 2 != 0,
            forced: flags & 4 != 0,
            night_mode: flags & 8 != 0,
            staircase: flags & 16 != 0,
            dimming: flags & 32 != 0,
            local_override: flags & 64 != 0,
            failure: flags & 128 != 0,
        };
        status.validate(actuator)?;
        Ok(status)
    }

    fn validate(&self, actuator: LightingActuator) -> Result<(), CodecError> {
        if actuator == LightingActuator::Switch
            && (self.dimming || matches!(self.level, Some(1..=254)))
        {
            return Err(CodecError::OutsideRange);
        }
        Ok(())
    }

    pub fn encode(self, actuator: LightingActuator) -> Result<[u8; 2], CodecError> {
        self.validate(actuator)?;
        let bits = [
            self.level.is_some(),
            self.locked,
            self.forced,
            self.night_mode,
            self.staircase,
            self.dimming,
            self.local_override,
            self.failure,
        ];
        let flags = bits
            .iter()
            .enumerate()
            .fold(0, |result, (i, flag)| result | (u8::from(*flag) << i));
        Ok([self.level.unwrap_or(0), flags])
    }
}

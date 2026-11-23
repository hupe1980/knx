//! Commissioned HVAC modes and configuration values from KNX DPT §§4.2–4.7.
//! DPT 31.101 is an E-mode push-button parameter. Use it only for configuration.

use crate::{CodecError, codecs::bytes, domain::wire_enum};

wire_enum!("1.100", HeatCool {
    Cooling = 0,
    Heating = 1,
});

wire_enum!("20.100", Fuel {
    Automatic = 0,
    Oil = 1,
    Gas = 2,
    Solid = 3,
});

wire_enum!("20.101", Burner {
    SingleStage = 1,
    TwoStage = 2,
    Modulating = 3,
});

wire_enum!("20.103", HotWaterMode {
    Auto = 0,
    LegionellaProtection = 1,
    Normal = 2,
    Reduced = 3,
    OffFrostProtection = 4,
});

wire_enum!("20.104", LoadPriority {
    None = 0,
    Shift = 1,
    Absolute = 2,
});

wire_enum!("20.105", ControllerMode {
    Auto = 0,
    Heat = 1,
    MorningWarmup = 2,
    Cool = 3,
    NightPurge = 4,
    Precool = 5,
    Off = 6,
    Test = 7,
    EmergencyHeat = 8,
    FanOnly = 9,
    FreeCooling = 10,
    Ice = 11,
    MaximumHeating = 12,
    EconomicHeatCool = 13,
    Dehumidify = 14,
    Calibration = 15,
    EmergencyCool = 16,
    EmergencySteam = 17,
    NoDemand = 20,
});

wire_enum!("20.106", EmergencyMode {
    Normal = 0,
    Pressurize = 1,
    Depressurize = 2,
    Purge = 3,
    Shutdown = 4,
    Fire = 5,
});

wire_enum!("20.107", Changeover {
    Automatic = 0,
    CoolingOnly = 1,
    HeatingOnly = 2,
});

wire_enum!("20.108", ValveMode {
    HeatStageA = 1,
    HeatStageB = 2,
    CoolStageA = 3,
    CoolStageB = 4,
    Changeover = 5,
});

wire_enum!("20.109", DamperMode {
    FreshAir = 1,
    SupplyAir = 2,
    DischargeAir = 3,
    ExtractAir = 4,
});

wire_enum!("20.110", HeaterMode {
    StageAOnOff = 1,
    StageAProportional = 2,
    StageBProportional = 3,
});

wire_enum!("20.111", FanMode {
    Stopped = 0,
    Continuous = 1,
    Intermittent = 2,
});

wire_enum!("20.112", ControllerRole {
    Autonomous = 0,
    Master = 1,
    Slave = 2,
});

wire_enum!("20.113", RoomSetpoint {
    Normal = 0,
    Alternative = 1,
    BuildingProtection = 2,
});

wire_enum!("20.114", MeterDevice {
    Other = 0,
    Oil = 1,
    Electricity = 2,
    Gas = 3,
    Heat = 4,
    Steam = 5,
    WarmWater = 6,
    Water = 7,
    HeatCostAllocator = 8,
    CoolingOutlet = 10,
    CoolingInlet = 11,
    HeatInlet = 12,
    HeatAndCool = 13,
    ElectricityBreaker = 32,
    SupplyValve = 33,
    WasteWater = 40,
    Garbage = 41,
    Void = 255,
});

wire_enum!("20.115", HumidityMode {
    Inactive = 0,
    Humidify = 1,
    Dehumidify = 2,
});

wire_enum!("20.120", DamperType {
    AirDamper = 1,
    VariableAirVolume = 2,
});

wire_enum!("20.121", BackupMode {
    BackupValue = 0,
    RetainLastState = 1,
});

wire_enum!("20.122", Synchronization {
    Unchanged = 0,
    CloseOnce = 1,
    OpenOnce = 2,
});

wire_enum!("23.102", PushButtonAction {
    ComfortEconomy = 0,
    ComfortOnly = 1,
    EconomyOnly = 2,
    ProtectionAuto = 3,
});

wire_enum!("31.101", PushButtonParameter {
    ComfortEconomy = 0,
    ComfortOnly = 1,
    EconomyOnly = 2,
    ProtectionAuto = 3,
    ProtectionOnly = 4,
    AutoOnly = 5,
    StandbyOnly = 6,
    ComfortStandby = 7,
});

use crate::HvacMode;
use crate::codecs::{decode_float16, encode_float16};
use std::num::NonZeroU16;

/// DPT 206.100/.102. Delay and mode can independently be unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeDelay<T> {
    pub minutes: Option<NonZeroU16>,
    pub mode: Option<T>,
}

macro_rules! mode_delay {
    ($mode:ty) => {
        impl ModeDelay<$mode> {
            pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
                let [hi, lo, code] = bytes(payload)?;
                Ok(Self {
                    minutes: NonZeroU16::new(u16::from_be_bytes([hi, lo])),
                    mode: if code == 0 {
                        None
                    } else {
                        Some(<$mode>::decode(&[code])?)
                    },
                })
            }
            pub fn encode(self) -> Result<[u8; 3], CodecError> {
                let [hi, lo] = self.minutes.map_or(0, NonZeroU16::get).to_be_bytes();
                let code = match self.mode {
                    None => 0,
                    Some(mode) if mode.encode()[0] != 0 => mode.encode()[0],
                    _ => return Err(CodecError::OutsideRange),
                };
                Ok([hi, lo, code])
            }
        }
    };
}
mode_delay!(HvacMode);
mode_delay!(HotWaterMode);

wire_enum!("206.104", Occupancy { Occupied=0, Standby=1, Unoccupied=2 });
wire_enum!("206.105", BuildingUse { InUse=0, Unused=1, Protection=2 });

/// A future value whose positive delay is known. Zero delay means no next value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scheduled<T> {
    pub minutes: NonZeroU16,
    pub value: T,
}

macro_rules! scheduled_mode {
    ($mode:ty) => {
        impl Scheduled<$mode> {
            pub fn decode(payload: &[u8]) -> Result<Option<Self>, CodecError> {
                let [hi, lo, code] = bytes(payload)?;
                let Some(minutes) = NonZeroU16::new(u16::from_be_bytes([hi, lo])) else {
                    return Ok(None);
                };
                Ok(Some(Self {
                    minutes,
                    value: <$mode>::decode(&[code])?,
                }))
            }
            pub fn encode(input: Option<Self>) -> [u8; 3] {
                let Some(input) = input else { return [0; 3] };
                let [hi, lo] = input.minutes.get().to_be_bytes();
                [hi, lo, input.value.encode()[0]]
            }
        }
    };
}
scheduled_mode!(Occupancy);
scheduled_mode!(BuildingUse);

/// Absolute temperature in exact increments of 0.02 °C (−273 to 655.34 °C).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemperatureTicks(i16);
impl TemperatureTicks {
    pub fn new(ticks: i16) -> Result<Self, CodecError> {
        if ticks < -13_650 {
            Err(CodecError::OutsideRange)
        } else {
            Ok(Self(ticks))
        }
    }
    pub const fn ticks(self) -> i16 {
        self.0
    }
    pub fn celsius(self) -> f64 {
        f64::from(self.0) / 50.0
    }
}

impl Scheduled<TemperatureTicks> {
    /// DPT 220.100. Ignore the unused temperature when the delay is zero.
    pub fn decode(payload: &[u8]) -> Result<Option<Self>, CodecError> {
        let [hi, lo, th, tl] = bytes(payload)?;
        let Some(minutes) = NonZeroU16::new(u16::from_be_bytes([hi, lo])) else {
            return Ok(None);
        };
        Ok(Some(Self {
            minutes,
            value: TemperatureTicks::new(i16::from_be_bytes([th, tl]))?,
        }))
    }
    pub fn encode(input: Option<Self>) -> [u8; 4] {
        let Some(input) = input else { return [0; 4] };
        let [hi, lo] = input.minutes.get().to_be_bytes();
        let [th, tl] = input.value.ticks().to_be_bytes();
        [hi, lo, th, tl]
    }
}

macro_rules! fixed_setpoints {
    ($name:ident,$desc:literal,$count:literal,$size:literal,$min:expr) => {
        #[doc=$desc]
        #[derive(Debug, Clone, Copy, Eq, PartialEq)]
        pub struct $name(pub [i16; $count]);
        impl $name {
            pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
                let words = bytes::<$size>(payload)?;
                let ticks =
                    std::array::from_fn(|i| i16::from_be_bytes([words[i * 2], words[i * 2 + 1]]));
                if ticks.iter().any(|v| *v < $min) {
                    return Err(CodecError::OutsideRange);
                }
                Ok(Self(ticks))
            }
            pub fn encode(self) -> Result<[u8; $size], CodecError> {
                if self.0.iter().any(|v| *v < $min) {
                    return Err(CodecError::OutsideRange);
                }
                let mut raw = [0; $size];
                for (slot, ticks) in raw.as_chunks_mut::<2>().0.iter_mut().zip(self.0) {
                    *slot = ticks.to_be_bytes();
                }
                Ok(raw)
            }
        }
    };
}
fixed_setpoints!(
    RoomShifts,
    "212.100: comfort, standby and economy shifts, in 0.02 K ticks.",
    3,
    6,
    -32767
);
fixed_setpoints!(
    RoomTemperatures,
    "212.101: comfort, standby and economy setpoints, in 0.02 °C ticks.",
    3,
    6,
    -13650
);
fixed_setpoints!(
    ProtectedRoomTemperatures,
    "213.100: comfort, standby, economy and building protection, in 0.02 °C ticks.",
    4,
    8,
    -13650
);
fixed_setpoints!(
    HotWaterTemperatures,
    "213.101: legionella protection, normal, reduced and frost protection, in 0.02 °C ticks.",
    4,
    8,
    -13650
);
fixed_setpoints!(
    ProtectedRoomShifts,
    "213.102: comfort, standby, economy and building protection shifts, in 0.02 K ticks.",
    4,
    8,
    -32767
);

/// 222.100 absolute setpoints or 222.101 relative shifts, ordered comfort, standby,
/// economy. An invalid field is independently represented as None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloatSetpoints {
    Celsius,
    KelvinShift,
}
impl FloatSetpoints {
    fn minimum(self) -> f64 {
        match self {
            Self::Celsius => -273.,
            Self::KelvinShift => -671_088.64,
        }
    }
    pub fn decode(self, payload: &[u8]) -> Result<[Option<f64>; 3], CodecError> {
        let raw = bytes::<6>(payload)?;
        let mut fields = [None; 3];
        for (field, word) in fields.iter_mut().zip(raw.as_chunks::<2>().0) {
            if *word != [127, 255] {
                *field = Some(decode_float16(word, self.minimum())?);
            }
        }
        Ok(fields)
    }
    pub fn encode(self, fields: [Option<f64>; 3]) -> Result<[u8; 6], CodecError> {
        let mut raw = [0; 6];
        for (word, value) in raw.as_chunks_mut::<2>().0.iter_mut().zip(fields) {
            *word = match value {
                Some(v) => encode_float16(v, self.minimum())?,
                None => [127, 255],
            };
        }
        Ok(raw)
    }
}

impl ControllerMode {
    fn demand_code(self) -> Result<u8, CodecError> {
        match self.encode()[0] {
            n @ (0..=11 | 20) => Ok(n),
            _ => Err(CodecError::OutsideRange),
        }
    }
    fn from_demand(code: u8) -> Result<Self, CodecError> {
        let mode = Self::decode(&[code])?;
        mode.demand_code()?;
        Ok(mode)
    }
}

/// 211.100 energy demand in percent for a water terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaterDemand {
    pub percent: u8,
    pub controller: ControllerMode,
}
impl WaterDemand {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [percent, mode] = bytes(payload)?;
        if percent > 100 {
            return Err(CodecError::OutsideRange);
        }
        Ok(Self {
            percent,
            controller: ControllerMode::from_demand(mode)?,
        })
    }
    pub fn encode(self) -> Result<[u8; 2], CodecError> {
        if self.percent > 100 {
            return Err(CodecError::OutsideRange);
        }
        Ok([self.percent, self.controller.demand_code()?])
    }
}

/// 223.100: negative demand is heating; positive demand is cooling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AirDemand {
    pub percent: i8,
    pub controller: ControllerMode,
    pub emergency: EmergencyMode,
}
impl AirDemand {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [percent, mode, emergency] = bytes(payload)?;
        let percent = percent as i8;
        if !(-100..=100).contains(&percent) {
            return Err(CodecError::OutsideRange);
        }
        Ok(Self {
            percent,
            controller: ControllerMode::from_demand(mode)?,
            emergency: EmergencyMode::decode(&[emergency])?,
        })
    }
    pub fn encode(self) -> Result<[u8; 3], CodecError> {
        if !(-100..=100).contains(&self.percent) {
            return Err(CodecError::OutsideRange);
        }
        Ok([
            self.percent as u8,
            self.controller.demand_code()?,
            self.emergency.encode()[0],
        ])
    }
}

/// 224.100: independent cooling and heating supply-air setpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplyAirSetpoints {
    pub cooling: TemperatureTicks,
    pub heating: TemperatureTicks,
    pub controller: ControllerMode,
    pub emergency: EmergencyMode,
}
impl SupplyAirSetpoints {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [a, b, c, d, mode, emergency] = bytes(payload)?;
        Ok(Self {
            cooling: TemperatureTicks::new(i16::from_be_bytes([a, b]))?,
            heating: TemperatureTicks::new(i16::from_be_bytes([c, d]))?,
            controller: ControllerMode::from_demand(mode)?,
            emergency: EmergencyMode::decode(&[emergency])?,
        })
    }
    pub fn encode(self) -> Result<[u8; 6], CodecError> {
        let [a, b] = self.cooling.ticks().to_be_bytes();
        let [c, d] = self.heating.ticks().to_be_bytes();
        Ok([
            a,
            b,
            c,
            d,
            self.controller.demand_code()?,
            self.emergency.encode()[0],
        ])
    }
}

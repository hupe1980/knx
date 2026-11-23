//! LTE HVAC compound application values in KNX Interworking §§4.13–4.15 and 4.19–4.21.
//!
//! One temperature tick is 0.02 °C. A percentage field is a whole percentage point.
//! An absent value leaves the accompanying control or diagnostic flags available.
//! Standard-mode points use their separately commissioned datapoint types.
//!
//! ```
//! use knx::hvac_reports::HeatManagerStatus;
//! let status = HeatManagerStatus::decode(&[0x80, 0x00, 0b10])?;
//! assert!(status.fault);
//! assert_eq!(status.flow_temperature, None);
//! # Ok::<(),knx::CodecError>(())
//! ```

use crate::{
    CodecError,
    codecs::bytes,
    hvac::{Burner, TemperatureTicks},
    hvac_flags::FuelTypes,
};

fn percent(value: u8) -> Result<u8, CodecError> {
    if value <= 100 {
        Ok(value)
    } else {
        Err(CodecError::OutsideRange)
    }
}
macro_rules! percent_report {
    ($id:literal, $name:ident, $value:ident: $kind:ident; $($flag:ident = $bit:literal),+ ) => {
        #[doc=concat!("DPT ",$id,". Percentages use whole units in the range 0–100.")]
        #[derive(Debug,Clone,Copy,Eq,PartialEq,Default)]
        pub struct $name {
            pub $value:percent_report!(@type $kind),
            $(pub $flag:bool),+
        }
        impl $name {
            pub fn decode(payload:&[u8])->Result<Self,CodecError>{
                let [value,flags]=bytes(payload)?;
                Ok(Self{ $value:percent_report!(@decode $kind,value,flags), $($flag:flags&(1<<$bit)!=0),+ })
            }
            pub fn encode(self)->Result<[u8;2],CodecError>{
                let (value,valid)=percent_report!(@encode $kind,self.$value);
                Ok([value,valid $(|u8::from(self.$flag)<<$bit)+])
            }
        }
    };
    (@type optional)=>{Option<u8>};
    (@type required)=>{u8};
    (@decode optional,$v:ident,$f:ident)=>{($f&1!=0).then(||percent($v)).transpose()?};
    (@decode required,$v:ident,$f:ident)=>{percent($v)?};
    (@encode optional,$v:expr)=>{($v.map(percent).transpose()?.unwrap_or_default(),u8::from($v.is_some()))};
    (@encode required,$v:expr)=>{(percent($v)?,0)};
}

percent_report!("207.100", BurnerStatus, power_percent: optional; fault=1, stage_one_active=2, stage_two_active=3);
percent_report!("207.101", LockSignal, reduction_percent: optional; critical=1);
percent_report!("207.102", BoilerDemand, demand_percent: required; stage_one_on=0, stage_two_on=1);
percent_report!("207.104", ActuatorDemand, position_percent: optional; absolute_priority=1, shift_priority=2, emergency=3);
percent_report!("207.105", ActuatorStatus, position_percent: required; failure=0, manual_override=1, calibrating=2, valve_kick=3, synchronizing=4);

fn read_temperature(
    high: u8,
    low: u8,
    valid: bool,
) -> Result<Option<TemperatureTicks>, CodecError> {
    valid
        .then(|| TemperatureTicks::new(i16::from_be_bytes([high, low])))
        .transpose()
}
fn write_temperature(value: Option<TemperatureTicks>) -> [u8; 2] {
    value.map_or(0, TemperatureTicks::ticks).to_be_bytes()
}

macro_rules! temperature_report {
    ($id:literal,$name:ident,$value:ident; $($flag:ident=$bit:literal),+) => {
        #[doc=concat!("DPT ",$id,". The optional temperature uses signed 0.02 °C ticks.")]
        #[derive(Debug,Copy,Clone,Eq,PartialEq,Default)]
        pub struct $name {pub $value:Option<TemperatureTicks>, $(pub $flag:bool),+}
        impl $name {
            pub fn decode(payload:&[u8])->Result<Self,CodecError>{
                let [hi,lo,flags]=bytes(payload)?;
                Ok(Self{$value:read_temperature(hi,lo,flags&1!=0)?, $($flag:flags&(1<<$bit)!=0),+})
            }
            pub fn encode(self)->[u8;3]{
                let [hi,lo]=write_temperature(self.$value);
                [hi,lo,u8::from(self.$value.is_some()) $(|u8::from(self.$flag)<<$bit)+]
            }
        }
    };
}
temperature_report!("209.100",HeatManagerStatus,flow_temperature; fault=1, summer_mode=2, permanently_off=3, no_heat=4);
temperature_report!("209.101",RoomDemand,room_temperature; absolute_priority=1, shift_priority=2, emergency=3);
temperature_report!("209.102",CoolingManagerStatus,flow_temperature; fault=1, permanently_off=2, no_cooling=3);
temperature_report!("209.103",WaterControllerStatus,water_temperature; fault=1, controller_running=2);

/// DPT 210.100. Legionella protection may be requested only as part of a DHW request.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct FlowDemand {
    pub flow_temperature: Option<TemperatureTicks>,
    pub absolute_priority: bool,
    pub shift_priority: bool,
    pub maximum_limit: bool,
    pub minimum_limit: bool,
    pub hot_water: bool,
    pub room: bool,
    pub ventilation: bool,
    pub auxiliary: bool,
    pub pump: bool,
    pub emergency: bool,
    pub hot_water_legionella: bool,
}
impl FlowDemand {
    fn validate(self) -> Result<Self, CodecError> {
        if self.hot_water_legionella && !self.hot_water {
            Err(CodecError::OutsideRange)
        } else {
            Ok(self)
        }
    }
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [hi, lo, h, l] = bytes(payload)?;
        let f = u16::from_be_bytes([h, l]);
        Self {
            flow_temperature: read_temperature(hi, lo, f & 1 != 0)?,
            absolute_priority: f & 2 != 0,
            shift_priority: f & 4 != 0,
            maximum_limit: f & 8 != 0,
            minimum_limit: f & 16 != 0,
            hot_water: f & 32 != 0,
            room: f & 64 != 0,
            ventilation: f & 128 != 0,
            auxiliary: f & 256 != 0,
            pump: f & 512 != 0,
            emergency: f & 1024 != 0,
            hot_water_legionella: f & 2048 != 0,
        }
        .validate()
    }
    pub fn encode(self) -> Result<[u8; 4], CodecError> {
        self.validate()?;
        let [hi, lo] = write_temperature(self.flow_temperature);
        let high = u8::from(self.auxiliary)
            | u8::from(self.pump) << 1
            | u8::from(self.emergency) << 2
            | u8::from(self.hot_water_legionella) << 3;
        let low = u8::from(self.flow_temperature.is_some())
            | u8::from(self.absolute_priority) << 1
            | u8::from(self.shift_priority) << 2
            | u8::from(self.maximum_limit) << 3
            | u8::from(self.minimum_limit) << 4
            | u8::from(self.hot_water) << 5
            | u8::from(self.room) << 6
            | u8::from(self.ventilation) << 7;
        Ok([hi, lo, high, low])
    }
}
/// DPT 214.100. Heat producer manager demand with stage enable and forced mode flags.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct HeatPowerDemand {
    pub flow_temperature: Option<TemperatureTicks>,
    pub limit_percent: u8,
    pub stage_one_enabled: bool,
    pub stage_one_forced: bool,
    pub stage_two_enabled: bool,
    pub stage_two_forced: bool,
    pub boiler_pump_enabled: bool,
}
impl HeatPowerDemand {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [hi, lo, limit, f] = bytes(payload)?;
        Ok(Self {
            flow_temperature: read_temperature(hi, lo, f & 1 != 0)?,
            limit_percent: percent(limit)?,
            stage_one_enabled: f & 2 != 0,
            stage_one_forced: f & 4 != 0,
            stage_two_enabled: f & 8 != 0,
            stage_two_forced: f & 16 != 0,
            boiler_pump_enabled: f & 32 != 0,
        })
    }
    pub fn encode(self) -> Result<[u8; 4], CodecError> {
        let [hi, lo] = write_temperature(self.flow_temperature);
        let f = u8::from(self.flow_temperature.is_some())
            | u8::from(self.stage_one_enabled) << 1
            | u8::from(self.stage_one_forced) << 2
            | u8::from(self.stage_two_enabled) << 3
            | u8::from(self.stage_two_forced) << 4
            | u8::from(self.boiler_pump_enabled) << 5;
        Ok([hi, lo, percent(self.limit_percent)?, f])
    }
}

/// DPT 214.101. The cooling-water pump enable is independent of both demand fields.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct CoolPowerDemand {
    pub flow_temperature: Option<TemperatureTicks>,
    pub limit_percent: Option<u8>,
    pub pump_enabled: bool,
}
impl CoolPowerDemand {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [hi, lo, limit, f] = bytes(payload)?;
        Ok(Self {
            flow_temperature: read_temperature(hi, lo, f & 1 != 0)?,
            limit_percent: (f & 2 != 0).then(|| percent(limit)).transpose()?,
            pump_enabled: f & 4 != 0,
        })
    }
    pub fn encode(self) -> Result<[u8; 4], CodecError> {
        let [hi, lo] = write_temperature(self.flow_temperature);
        let f = u8::from(self.flow_temperature.is_some())
            | u8::from(self.limit_percent.is_some()) << 1
            | u8::from(self.pump_enabled) << 2;
        Ok([
            hi,
            lo,
            self.limit_percent
                .map(percent)
                .transpose()?
                .unwrap_or_default(),
            f,
        ])
    }
}

macro_rules! plant_status {
    ($id:literal,$name:ident; $($flag:ident=$bit:literal),+) => {
        #[doc=concat!("DPT ",$id,". Boiler/chiller temperature and power each have a validity flag.")]
        #[derive(Debug,Copy,Clone,Default,PartialEq,Eq)]
        pub struct $name {pub temperature:Option<TemperatureTicks>,pub power_percent:Option<u8>, $(pub $flag:bool),+}
        impl $name {
            pub fn decode(payload:&[u8])->Result<Self,CodecError>{
                let [hi,lo,p,h,l]=bytes(payload)?;
                let f=u16::from_be_bytes([h,l]);
                Ok(Self{temperature:read_temperature(hi,lo,f&1!=0)?,power_percent:(f&2!=0).then(||percent(p)).transpose()?, $($flag:f&(1<<$bit)!=0),+})
            }
            pub fn encode(self)->Result<[u8;5],CodecError>{
                let [hi,lo]=write_temperature(self.temperature);
                let f=u16::from(self.temperature.is_some())|u16::from(self.power_percent.is_some())<<1 $(|u16::from(self.$flag)<<$bit)+;
                let [h,l]=f.to_be_bytes();
                Ok([hi,lo,self.power_percent.map(percent).transpose()?.unwrap_or_default(),h,l])
            }
        }
    }
}
plant_status!("215.100",BoilerStatus; fault=2,summer_mode=3,permanently_off=4,no_heat=5,stage_one_enabled=6,stage_two_enabled=7,request_next_stage=8,request_next_boiler=9,reduced_availability=10,chimney_sweep=11);
plant_status!("215.101",ChillerStatus; running=2,fault=3,permanently_off=4,request_next_stage=5,request_next_chiller=6,reduced_availability=7);

/// DPT 216.100 specifies nominal power and burner stages. A single-stage burner sends 100% for its unused base-stage limit.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct HeatProducerSpec {
    pub nominal_kw: u16,
    pub base_stage_percent: u8,
    pub burner: Burner,
    pub fuels: FuelTypes,
}
impl HeatProducerSpec {
    fn validate(self) -> Result<Self, CodecError> {
        percent(self.base_stage_percent)?;
        if self.burner == Burner::SingleStage && self.base_stage_percent != 100 {
            return Err(CodecError::OutsideRange);
        }
        Ok(self)
    }
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [hi, lo, base, b, f] = bytes(payload)?;
        Self {
            nominal_kw: u16::from_be_bytes([hi, lo]),
            base_stage_percent: base,
            burner: Burner::decode(&[b])?,
            fuels: FuelTypes::decode(&[f & 7])?,
        }
        .validate()
    }
    pub fn encode(self) -> Result<[u8; 5], CodecError> {
        self.validate()?;
        let [hi, lo] = self.nominal_kw.to_be_bytes();
        Ok([
            hi,
            lo,
            self.base_stage_percent,
            self.burner.encode()[0],
            self.fuels.encode()[0],
        ])
    }
}

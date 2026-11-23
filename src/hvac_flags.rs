//! Structured HVAC bitsets in KNX Interworking §§4.4–4.5. Several represent
//! LTE compound properties. Use separate commissioned DPTs for their S-mode
//! equivalents where the profile specifies them. Optional fields depend on
//! device support. For a heating forcing signal, evaluate its attributes only
//! while `requested` is set.

use crate::{CodecError, codecs::bytes, system::flag_fields};

flag_fields!("21.100", HeatForcing, u8, 0; requested=0, protection=1, oversupply=2, overrun=3,
    hot_water_normal=4, hot_water_legionella_protection=5, room_comfort=6, room_max_flow=7);
flag_fields!("21.101", CoolForcing, u8, 0; requested=0);
flag_fields!("21.102", RoomHeatingStatus, u8, 0; fault=0, eco=1, flow_limited=2, return_limited=3,
    morning_boost=4, optimized_start=5, optimized_stop=6, summer=7);
flag_fields!("21.103", SolarHotWaterStatus, u8, 0; fault=0, solar_load_active=1, solar_load_sufficient=2);
flag_fields!("21.104", FuelTypes, u8, 0; oil=0, gas=1, solid=2);
flag_fields!("21.105", RoomCoolingStatus, u8, 0; fault=0);
flag_fields!("21.106", VentilationStatus, u8, 0; fault=0, fan_active=1, heating=2, cooling=3);
flag_fields!("22.100", HotWaterStatus, u16, 0; fault=0, load_active=1, legionella_protection=2, push_active=3,
    other_energy_source=4, solar_only=5, solar_support=6, setpoint_shift=7);
flag_fields!("22.101", RoomClimateStatus, u16, 0; fault=0, economy_heating=1, flow_limited=2, return_limited=3,
    heating_boost=4, optimized_start=5, optimized_stop=6, heating_disabled=7, heating_mode=8 => true, economy_cooling=9,
    precool=10, cooling_disabled=11, dew_point_alarm=12, frost_alarm=13, overheat_alarm=14);

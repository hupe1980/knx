#[path = "vectors/register_layout.rs"]
mod register_layout;
use knx::domain::dali::*;
use knx::hvac_flags::*;
use knx::hvac_reports as reports;
use knx::system::*;
#[path = "vectors/structured.rs"]
mod structured_vectors;
use knx::domain::*;
use knx::text_time::*;
use knx::*;
use std::collections::HashSet;

const CATALOG: &str = include_str!("../site/static/standards.csv");

// Each catalog claim must resolve to an executable test for exactly that ID and direction.
const ENCODE: usize = 6;
const DECODE: usize = 7;

macro_rules! vectors {
    ($evidence:ident; $(($id:literal, $direction:ident, $test:ident, $actual:expr, $expected:expr)),+ $(,)?) => {
        $(
            #[test]
            fn $test() { assert_eq!($actual, $expected); }
        )+
        const $evidence: &[(&str, usize, &str)] = &[
            $(($id, $direction, concat!("tests/inventory.rs::", stringify!($test))),)+
        ];
    };
}

// Each catalog entry gets separately runnable encode and decode evidence.
macro_rules! family_vectors {
    ($family:ident, $descriptor:ident, $bits:expr, $sample:expr, $(($id:literal, $tx:ident, $rx:ident)),+ $(,)?) => {
        mod $family {
            use super::*;
            $(
                #[test] fn $tx() {
                    let dpt = $descriptor::from_dpt($id).unwrap();
                    assert_eq!(dpt.encode($sample), Ok($bits));
                }
                #[test] fn $rx() {
                    let dpt = $descriptor::from_dpt($id).unwrap();
                    assert_eq!(dpt.decode(&$bits), Ok($sample));
                }
            )+
            pub(super) const EVIDENCE: &[(&str, usize, &str)] = &[
                $(($id, ENCODE, concat!("tests/inventory.rs::", stringify!($family), "::", stringify!($tx))),
                  ($id, DECODE, concat!("tests/inventory.rs::", stringify!($family), "::", stringify!($rx))),)+
            ];
        }
    };
}

family_vectors!(
    short_float,
    Float16Dpt,
    [0x04, 0xe2],
    12.5,
    ("9.002", dpt_9_002_tx, dpt_9_002_rx),
    ("9.003", dpt_9_003_tx, dpt_9_003_rx),
    ("9.004", dpt_9_004_tx, dpt_9_004_rx),
    ("9.005", dpt_9_005_tx, dpt_9_005_rx),
    ("9.006", dpt_9_006_tx, dpt_9_006_rx),
    ("9.007", dpt_9_007_tx, dpt_9_007_rx),
    ("9.008", dpt_9_008_tx, dpt_9_008_rx),
    ("9.009", dpt_9_009_tx, dpt_9_009_rx),
    ("9.010", dpt_9_010_tx, dpt_9_010_rx),
    ("9.011", dpt_9_011_tx, dpt_9_011_rx),
    ("9.020", dpt_9_020_tx, dpt_9_020_rx),
    ("9.021", dpt_9_021_tx, dpt_9_021_rx),
    ("9.022", dpt_9_022_tx, dpt_9_022_rx),
    ("9.023", dpt_9_023_tx, dpt_9_023_rx),
    ("9.024", dpt_9_024_tx, dpt_9_024_rx),
    ("9.025", dpt_9_025_tx, dpt_9_025_rx),
    ("9.026", dpt_9_026_tx, dpt_9_026_rx),
    ("9.027", dpt_9_027_tx, dpt_9_027_rx),
    ("9.028", dpt_9_028_tx, dpt_9_028_rx),
    ("9.029", dpt_9_029_tx, dpt_9_029_rx),
    ("9.030", dpt_9_030_tx, dpt_9_030_rx),
);

family_vectors!(
    long_float,
    Float32Dpt,
    [0xc0, 0x20, 0, 0],
    -2.5,
    ("14.000", dpt_14_000_tx, dpt_14_000_rx),
    ("14.001", dpt_14_001_tx, dpt_14_001_rx),
    ("14.002", dpt_14_002_tx, dpt_14_002_rx),
    ("14.003", dpt_14_003_tx, dpt_14_003_rx),
    ("14.004", dpt_14_004_tx, dpt_14_004_rx),
    ("14.005", dpt_14_005_tx, dpt_14_005_rx),
    ("14.006", dpt_14_006_tx, dpt_14_006_rx),
    ("14.007", dpt_14_007_tx, dpt_14_007_rx),
    ("14.008", dpt_14_008_tx, dpt_14_008_rx),
    ("14.009", dpt_14_009_tx, dpt_14_009_rx),
    ("14.010", dpt_14_010_tx, dpt_14_010_rx),
    ("14.011", dpt_14_011_tx, dpt_14_011_rx),
    ("14.012", dpt_14_012_tx, dpt_14_012_rx),
    ("14.013", dpt_14_013_tx, dpt_14_013_rx),
    ("14.014", dpt_14_014_tx, dpt_14_014_rx),
    ("14.015", dpt_14_015_tx, dpt_14_015_rx),
    ("14.016", dpt_14_016_tx, dpt_14_016_rx),
    ("14.017", dpt_14_017_tx, dpt_14_017_rx),
    ("14.018", dpt_14_018_tx, dpt_14_018_rx),
    ("14.019", dpt_14_019_tx, dpt_14_019_rx),
    ("14.020", dpt_14_020_tx, dpt_14_020_rx),
    ("14.021", dpt_14_021_tx, dpt_14_021_rx),
    ("14.022", dpt_14_022_tx, dpt_14_022_rx),
    ("14.023", dpt_14_023_tx, dpt_14_023_rx),
    ("14.024", dpt_14_024_tx, dpt_14_024_rx),
    ("14.025", dpt_14_025_tx, dpt_14_025_rx),
    ("14.026", dpt_14_026_tx, dpt_14_026_rx),
    ("14.027", dpt_14_027_tx, dpt_14_027_rx),
    ("14.028", dpt_14_028_tx, dpt_14_028_rx),
    ("14.029", dpt_14_029_tx, dpt_14_029_rx),
    ("14.030", dpt_14_030_tx, dpt_14_030_rx),
    ("14.031", dpt_14_031_tx, dpt_14_031_rx),
    ("14.032", dpt_14_032_tx, dpt_14_032_rx),
    ("14.033", dpt_14_033_tx, dpt_14_033_rx),
    ("14.034", dpt_14_034_tx, dpt_14_034_rx),
    ("14.035", dpt_14_035_tx, dpt_14_035_rx),
    ("14.036", dpt_14_036_tx, dpt_14_036_rx),
    ("14.037", dpt_14_037_tx, dpt_14_037_rx),
    ("14.038", dpt_14_038_tx, dpt_14_038_rx),
    ("14.039", dpt_14_039_tx, dpt_14_039_rx),
    ("14.040", dpt_14_040_tx, dpt_14_040_rx),
    ("14.041", dpt_14_041_tx, dpt_14_041_rx),
    ("14.042", dpt_14_042_tx, dpt_14_042_rx),
    ("14.043", dpt_14_043_tx, dpt_14_043_rx),
    ("14.044", dpt_14_044_tx, dpt_14_044_rx),
    ("14.045", dpt_14_045_tx, dpt_14_045_rx),
    ("14.046", dpt_14_046_tx, dpt_14_046_rx),
    ("14.047", dpt_14_047_tx, dpt_14_047_rx),
    ("14.048", dpt_14_048_tx, dpt_14_048_rx),
    ("14.049", dpt_14_049_tx, dpt_14_049_rx),
    ("14.050", dpt_14_050_tx, dpt_14_050_rx),
    ("14.051", dpt_14_051_tx, dpt_14_051_rx),
    ("14.052", dpt_14_052_tx, dpt_14_052_rx),
    ("14.053", dpt_14_053_tx, dpt_14_053_rx),
    ("14.054", dpt_14_054_tx, dpt_14_054_rx),
    ("14.055", dpt_14_055_tx, dpt_14_055_rx),
    ("14.057", dpt_14_057_tx, dpt_14_057_rx),
    ("14.058", dpt_14_058_tx, dpt_14_058_rx),
    ("14.059", dpt_14_059_tx, dpt_14_059_rx),
    ("14.060", dpt_14_060_tx, dpt_14_060_rx),
    ("14.061", dpt_14_061_tx, dpt_14_061_rx),
    ("14.062", dpt_14_062_tx, dpt_14_062_rx),
    ("14.063", dpt_14_063_tx, dpt_14_063_rx),
    ("14.064", dpt_14_064_tx, dpt_14_064_rx),
    ("14.065", dpt_14_065_tx, dpt_14_065_rx),
    ("14.066", dpt_14_066_tx, dpt_14_066_rx),
    ("14.067", dpt_14_067_tx, dpt_14_067_rx),
    ("14.068", dpt_14_068_tx, dpt_14_068_rx),
    ("14.069", dpt_14_069_tx, dpt_14_069_rx),
    ("14.070", dpt_14_070_tx, dpt_14_070_rx),
    ("14.071", dpt_14_071_tx, dpt_14_071_rx),
    ("14.072", dpt_14_072_tx, dpt_14_072_rx),
    ("14.073", dpt_14_073_tx, dpt_14_073_rx),
    ("14.074", dpt_14_074_tx, dpt_14_074_rx),
    ("14.075", dpt_14_075_tx, dpt_14_075_rx),
    ("14.076", dpt_14_076_tx, dpt_14_076_rx),
    ("14.077", dpt_14_077_tx, dpt_14_077_rx),
    ("14.078", dpt_14_078_tx, dpt_14_078_rx),
    ("14.079", dpt_14_079_tx, dpt_14_079_rx),
    ("14.080", dpt_14_080_tx, dpt_14_080_rx),
);

vectors!(MBUS_LAYOUT;
    ("230.1000", ENCODE, mbus_address_tx, MBusAddress{manufacturer:0x1596, identification:0x12345678,version:3,medium:0x1b}.encode(), [0x15,0x96,0x12,0x34,0x56,0x78,3,0x1b]),
    ("230.1000", DECODE, mbus_address_rx, MBusAddress::decode(&[0x15,0x96,0x12,0x34,0x56,0x78,3,0x1b]), Ok(MBusAddress{manufacturer:0x1596, identification:0x12345678,version:3,medium:0x1b})),
);

vectors!(EVIDENCE;
    ("207.100", ENCODE, report_burner_status_tx, reports::BurnerStatus{power_percent:Some(25),fault:true,stage_two_active:true,..Default::default()}.encode(), Ok([25,11])),
    ("207.100", DECODE, report_burner_status_rx, reports::BurnerStatus::decode(&[25,11]), Ok(reports::BurnerStatus{power_percent:Some(25),fault:true,stage_two_active:true,..Default::default()})),
    ("207.101", ENCODE, report_lock_signal_tx, reports::LockSignal{reduction_percent:Some(100),critical:true}.encode(), Ok([100,3])),
    ("207.101", DECODE, report_lock_signal_rx, reports::LockSignal::decode(&[100,3]), Ok(reports::LockSignal{reduction_percent:Some(100),critical:true})),
    ("207.102", ENCODE, report_boiler_demand_tx, reports::BoilerDemand{demand_percent:15,stage_two_on:true,..Default::default()}.encode(), Ok([15,2])),
    ("207.102", DECODE, report_boiler_demand_rx, reports::BoilerDemand::decode(&[15,2]), Ok(reports::BoilerDemand{demand_percent:15,stage_two_on:true,..Default::default()})),
    ("207.104", ENCODE, report_actuator_demand_tx, reports::ActuatorDemand{position_percent:Some(90),absolute_priority:true,emergency:true,..Default::default()}.encode(), Ok([90,11])),
    ("207.104", DECODE, report_actuator_demand_rx, reports::ActuatorDemand::decode(&[90,11]), Ok(reports::ActuatorDemand{position_percent:Some(90),absolute_priority:true,emergency:true,..Default::default()})),
    ("207.105", ENCODE, report_actuator_status_tx, reports::ActuatorStatus{position_percent:29,synchronizing:true,valve_kick:true,..Default::default()}.encode(), Ok([29,24])),
    ("207.105", DECODE, report_actuator_status_rx, reports::ActuatorStatus::decode(&[29,24]), Ok(reports::ActuatorStatus{position_percent:29,synchronizing:true,valve_kick:true,..Default::default()})),
    ("209.100", ENCODE, report_heat_manager_status_tx, reports::HeatManagerStatus{flow_temperature:Some(hvac::TemperatureTicks::new(1000).unwrap()),fault:true,no_heat:true,..Default::default()}.encode(), [3,232,19]),
    ("209.100", DECODE, report_heat_manager_status_rx, reports::HeatManagerStatus::decode(&[3,232,19]), Ok(reports::HeatManagerStatus{flow_temperature:Some(hvac::TemperatureTicks::new(1000).unwrap()),fault:true,no_heat:true,..Default::default()})),
    ("209.101", ENCODE, report_room_demand_tx, reports::RoomDemand{room_temperature:Some(hvac::TemperatureTicks::new(-50).unwrap()),shift_priority:true,..Default::default()}.encode(), [255,206,5]),
    ("209.101", DECODE, report_room_demand_rx, reports::RoomDemand::decode(&[255,206,5]), Ok(reports::RoomDemand{room_temperature:Some(hvac::TemperatureTicks::new(-50).unwrap()),shift_priority:true,..Default::default()})),
    ("209.102", ENCODE, report_cooling_manager_status_tx, reports::CoolingManagerStatus{flow_temperature:None,permanently_off:true,no_cooling:true,..Default::default()}.encode(), [0,0,12]),
    ("209.102", DECODE, report_cooling_manager_status_rx, reports::CoolingManagerStatus::decode(&[0,0,12]), Ok(reports::CoolingManagerStatus{flow_temperature:None,permanently_off:true,no_cooling:true,..Default::default()})),
    ("209.103", ENCODE, report_water_controller_status_tx, reports::WaterControllerStatus{water_temperature:Some(hvac::TemperatureTicks::new(500).unwrap()),controller_running:true,..Default::default()}.encode(), [1,244,5]),
    ("209.103", DECODE, report_water_controller_status_rx, reports::WaterControllerStatus::decode(&[1,244,5]), Ok(reports::WaterControllerStatus{water_temperature:Some(hvac::TemperatureTicks::new(500).unwrap()),controller_running:true,..Default::default()})),
    ("210.100", ENCODE, report_flow_demand_tx, reports::FlowDemand{hot_water:true,hot_water_legionella:true,pump:true,..Default::default()}.encode(), Ok([0,0,10,32])),
    ("210.100", DECODE, report_flow_demand_rx, reports::FlowDemand::decode(&[0,0,10,32]), Ok(reports::FlowDemand{hot_water:true,hot_water_legionella:true,pump:true,..Default::default()})),
    ("214.100", ENCODE, report_heat_power_demand_tx, reports::HeatPowerDemand{limit_percent:99,stage_one_forced:true,boiler_pump_enabled:true,..Default::default()}.encode(), Ok([0,0,99,36])),
    ("214.100", DECODE, report_heat_power_demand_rx, reports::HeatPowerDemand::decode(&[0,0,99,36]), Ok(reports::HeatPowerDemand{limit_percent:99,stage_one_forced:true,boiler_pump_enabled:true,..Default::default()})),
    ("214.101", ENCODE, report_cool_power_demand_tx, reports::CoolPowerDemand{limit_percent:Some(10),pump_enabled:true,..Default::default()}.encode(), Ok([0,0,10,6])),
    ("214.101", DECODE, report_cool_power_demand_rx, reports::CoolPowerDemand::decode(&[0,0,10,6]), Ok(reports::CoolPowerDemand{limit_percent:Some(10),pump_enabled:true,..Default::default()})),
    ("215.100", ENCODE, report_boiler_status_tx, reports::BoilerStatus{power_percent:Some(100),request_next_boiler:true,chimney_sweep:true,..Default::default()}.encode(), Ok([0,0,100,10,2])),
    ("215.100", DECODE, report_boiler_status_rx, reports::BoilerStatus::decode(&[0,0,100,10,2]), Ok(reports::BoilerStatus{power_percent:Some(100),request_next_boiler:true,chimney_sweep:true,..Default::default()})),
    ("215.101", ENCODE, report_chiller_status_tx, reports::ChillerStatus{temperature:Some(hvac::TemperatureTicks::new(250).unwrap()),fault:true,request_next_chiller:true,..Default::default()}.encode(), Ok([0,250,0,0,73])),
    ("215.101", DECODE, report_chiller_status_rx, reports::ChillerStatus::decode(&[0,250,0,0,73]), Ok(reports::ChillerStatus{temperature:Some(hvac::TemperatureTicks::new(250).unwrap()),fault:true,request_next_chiller:true,..Default::default()})),
    ("216.100", ENCODE, report_heat_producer_spec_tx, reports::HeatProducerSpec{nominal_kw:258,base_stage_percent:100,burner:hvac::Burner::SingleStage,fuels:FuelTypes{oil:true,gas:true,solid:false}}.encode(), Ok([1,2,100,1,3])),
    ("216.100", DECODE, report_heat_producer_spec_rx, reports::HeatProducerSpec::decode(&[1,2,100,1,3]), Ok(reports::HeatProducerSpec{nominal_kw:258,base_stage_percent:100,burner:hvac::Burner::SingleStage,fuels:FuelTypes{oil:true,gas:true,solid:false}})),
    ("206.100", ENCODE, hvac_206_100_tx, hvac::ModeDelay::<HvacMode>{minutes:std::num::NonZeroU16::new(258),mode:Some(HvacMode::Economy)}.encode(), Ok([1,2,3])),
    ("206.100", DECODE, hvac_206_100_rx, hvac::ModeDelay::<HvacMode>::decode(&[1,2,3]), Ok(hvac::ModeDelay::<HvacMode>{minutes:std::num::NonZeroU16::new(258),mode:Some(HvacMode::Economy)})),
    ("206.102", ENCODE, hvac_206_102_tx, hvac::ModeDelay::<hvac::HotWaterMode>{minutes:std::num::NonZeroU16::new(258),mode:Some(hvac::HotWaterMode::Reduced)}.encode(), Ok([1,2,3])),
    ("206.102", DECODE, hvac_206_102_rx, hvac::ModeDelay::<hvac::HotWaterMode>::decode(&[1,2,3]), Ok(hvac::ModeDelay::<hvac::HotWaterMode>{minutes:std::num::NonZeroU16::new(258),mode:Some(hvac::HotWaterMode::Reduced)})),
    ("206.104", ENCODE, hvac_206_104_tx, Ok::<_,CodecError>(hvac::Scheduled::<hvac::Occupancy>::encode(Some(hvac::Scheduled::<hvac::Occupancy>{minutes:std::num::NonZeroU16::new(258).unwrap(),value:hvac::Occupancy::Unoccupied}))), Ok([1,2,2])),
    ("206.104", DECODE, hvac_206_104_rx, hvac::Scheduled::<hvac::Occupancy>::decode(&[1,2,2]), Ok(Some(hvac::Scheduled::<hvac::Occupancy>{minutes:std::num::NonZeroU16::new(258).unwrap(),value:hvac::Occupancy::Unoccupied}))),
    ("206.105", ENCODE, hvac_206_105_tx, Ok::<_,CodecError>(hvac::Scheduled::<hvac::BuildingUse>::encode(Some(hvac::Scheduled::<hvac::BuildingUse>{minutes:std::num::NonZeroU16::new(258).unwrap(),value:hvac::BuildingUse::Unused}))), Ok([1,2,1])),
    ("206.105", DECODE, hvac_206_105_rx, hvac::Scheduled::<hvac::BuildingUse>::decode(&[1,2,1]), Ok(Some(hvac::Scheduled::<hvac::BuildingUse>{minutes:std::num::NonZeroU16::new(258).unwrap(),value:hvac::BuildingUse::Unused}))),
    ("220.100", ENCODE, hvac_220_100_tx, Ok::<_,CodecError>(hvac::Scheduled::<hvac::TemperatureTicks>::encode(Some(hvac::Scheduled::<hvac::TemperatureTicks>{minutes:std::num::NonZeroU16::new(258).unwrap(),value:hvac::TemperatureTicks::new(-50).unwrap()}))), Ok([1,2,255,206])),
    ("220.100", DECODE, hvac_220_100_rx, hvac::Scheduled::<hvac::TemperatureTicks>::decode(&[1,2,255,206]), Ok(Some(hvac::Scheduled::<hvac::TemperatureTicks>{minutes:std::num::NonZeroU16::new(258).unwrap(),value:hvac::TemperatureTicks::new(-50).unwrap()}))),
    ("212.100", ENCODE, hvac_212_100_tx, hvac::RoomShifts([-50,0,1000]).encode(), Ok([255,206,0,0,3,232])),
    ("212.100", DECODE, hvac_212_100_rx, hvac::RoomShifts::decode(&[255,206,0,0,3,232]), Ok(hvac::RoomShifts([-50,0,1000]))),
    ("212.101", ENCODE, hvac_212_101_tx, hvac::RoomTemperatures([-50,0,1000]).encode(), Ok([255,206,0,0,3,232])),
    ("212.101", DECODE, hvac_212_101_rx, hvac::RoomTemperatures::decode(&[255,206,0,0,3,232]), Ok(hvac::RoomTemperatures([-50,0,1000]))),
    ("213.100", ENCODE, hvac_213_100_tx, hvac::ProtectedRoomTemperatures([-50,0,1000,5]).encode(), Ok([255,206,0,0,3,232,0,5])),
    ("213.100", DECODE, hvac_213_100_rx, hvac::ProtectedRoomTemperatures::decode(&[255,206,0,0,3,232,0,5]), Ok(hvac::ProtectedRoomTemperatures([-50,0,1000,5]))),
    ("213.101", ENCODE, hvac_213_101_tx, hvac::HotWaterTemperatures([-50,0,1000,5]).encode(), Ok([255,206,0,0,3,232,0,5])),
    ("213.101", DECODE, hvac_213_101_rx, hvac::HotWaterTemperatures::decode(&[255,206,0,0,3,232,0,5]), Ok(hvac::HotWaterTemperatures([-50,0,1000,5]))),
    ("213.102", ENCODE, hvac_213_102_tx, hvac::ProtectedRoomShifts([-50,0,1000,5]).encode(), Ok([255,206,0,0,3,232,0,5])),
    ("213.102", DECODE, hvac_213_102_rx, hvac::ProtectedRoomShifts::decode(&[255,206,0,0,3,232,0,5]), Ok(hvac::ProtectedRoomShifts([-50,0,1000,5]))),
    ("222.100", ENCODE, hvac_222_100_tx, hvac::FloatSetpoints::Celsius.encode([Some(20.0),None,Some(-1.0)]), Ok([7,208,127,255,135,156])),
    ("222.100", DECODE, hvac_222_100_rx, hvac::FloatSetpoints::Celsius.decode(&[7,208,127,255,135,156]), Ok([Some(20.0),None,Some(-1.0)])),
    ("222.101", ENCODE, hvac_222_101_tx, hvac::FloatSetpoints::KelvinShift.encode([Some(20.0),None,Some(-1.0)]), Ok([7,208,127,255,135,156])),
    ("222.101", DECODE, hvac_222_101_rx, hvac::FloatSetpoints::KelvinShift.decode(&[7,208,127,255,135,156]), Ok([Some(20.0),None,Some(-1.0)])),
    ("211.100", ENCODE, hvac_211_100_tx, hvac::WaterDemand{percent:70,controller:hvac::ControllerMode::Heat}.encode(), Ok([70,1])),
    ("211.100", DECODE, hvac_211_100_rx, hvac::WaterDemand::decode(&[70,1]), Ok(hvac::WaterDemand{percent:70,controller:hvac::ControllerMode::Heat})),
    ("223.100", ENCODE, hvac_223_100_tx, hvac::AirDemand{percent:-100,controller:hvac::ControllerMode::Heat,emergency:hvac::EmergencyMode::Normal}.encode(), Ok([156,1,0])),
    ("223.100", DECODE, hvac_223_100_rx, hvac::AirDemand::decode(&[156,1,0]), Ok(hvac::AirDemand{percent:-100,controller:hvac::ControllerMode::Heat,emergency:hvac::EmergencyMode::Normal})),
    ("224.100", ENCODE, hvac_224_100_tx, hvac::SupplyAirSetpoints{cooling:hvac::TemperatureTicks::new(1200).unwrap(),heating:hvac::TemperatureTicks::new(1000).unwrap(),controller:hvac::ControllerMode::NoDemand,emergency:hvac::EmergencyMode::Normal}.encode(), Ok([4,176,3,232,20,0])),
    ("224.100", DECODE, hvac_224_100_rx, hvac::SupplyAirSetpoints::decode(&[4,176,3,232,20,0]), Ok(hvac::SupplyAirSetpoints{cooling:hvac::TemperatureTicks::new(1200).unwrap(),heating:hvac::TemperatureTicks::new(1000).unwrap(),controller:hvac::ControllerMode::NoDemand,emergency:hvac::EmergencyMode::Normal})),
    ("20.1000", ENCODE, communication_mode_tx, CommunicationMode::TransportLayer.encode(), [6]),
    ("20.1000", DECODE, communication_mode_rx, CommunicationMode::decode(&[6]), Ok(CommunicationMode::TransportLayer)),
    ("20.1001", ENCODE, add_info_type_tx, AdditionalInfoType::ExtendedRelativeTimestamp.encode(), [6]),
    ("20.1001", DECODE, add_info_type_rx, AdditionalInfoType::decode(&[6]), Ok(AdditionalInfoType::ExtendedRelativeTimestamp)),
    ("20.1002", ENCODE, radio_mode_tx, RadioMode::BiBatSlave.encode(), [2]),
    ("20.1002", DECODE, radio_mode_rx, RadioMode::decode(&[2]), Ok(RadioMode::BiBatSlave)),
    ("20.1003", ENCODE, radio_filter_tx, RadioFilter::DomainAndSerialNumber.encode(), [3]),
    ("20.1003", DECODE, radio_filter_rx, RadioFilter::decode(&[3]), Ok(RadioFilter::DomainAndSerialNumber)),
    ("20.1004", ENCODE, system_medium_tx, MediumType::Ip.encode(), [5]),
    ("20.1004", DECODE, system_medium_rx, MediumType::decode(&[5]), Ok(MediumType::Ip)),
    ("20.1005", ENCODE, button_function_tx, ButtonFunction::ShutterUpDownSwitch.encode(), [42]),
    ("20.1005", DECODE, button_function_rx, ButtonFunction::decode(&[42]), Ok(ButtonFunction::ShutterUpDownSwitch)),
    ("21.1000", ENCODE, radio_modes_tx, RadioModeSupport{bibat_slave: true, ..Default::default()}.encode(), [5]),
    ("21.1000", DECODE, radio_modes_rx, RadioModeSupport::decode(&[5]), Ok(RadioModeSupport{bibat_slave: true, ..Default::default()})),
    ("21.1001", ENCODE, radio_filters_tx, RadioFilterSupport{domain_and_serial_number: true, ..Default::default()}.encode(), [4]),
    ("21.1001", DECODE, radio_filters_rx, RadioFilterSupport::decode(&[4]), Ok(RadioFilterSupport{domain_and_serial_number: true, ..Default::default()})),
    ("21.1002", ENCODE, security_report_tx, SecurityReport{failure_since_previous_report: true}.encode(), [1]),
    ("21.1002", DECODE, security_report_rx, SecurityReport::decode(&[1]), Ok(SecurityReport{failure_since_previous_report: true})),
    ("22.1000", ENCODE, supported_media_tx, SupportedMedia{ip: true, ..Default::default()}.encode(), [0,32]),
    ("22.1000", DECODE, supported_media_rx, SupportedMedia::decode(&[0,32]), Ok(SupportedMedia{ip: true, ..Default::default()})),
    ("21.100", ENCODE, heat_forcing_tx, HeatForcing{requested:true, hot_water_normal: true, ..Default::default()}.encode(), [17]),
    ("21.100", DECODE, heat_forcing_rx, HeatForcing::decode(&[17]), Ok(HeatForcing{requested:true, hot_water_normal: true, ..Default::default()})),
    ("21.101", ENCODE, cool_forcing_tx, CoolForcing{requested: true}.encode(), [1]),
    ("21.101", DECODE, cool_forcing_rx, CoolForcing::decode(&[1]), Ok(CoolForcing{requested: true})),
    ("21.102", ENCODE, room_heating_status_tx, RoomHeatingStatus{optimized_stop: true, ..Default::default()}.encode(), [64]),
    ("21.102", DECODE, room_heating_status_rx, RoomHeatingStatus::decode(&[64]), Ok(RoomHeatingStatus{optimized_stop: true, ..Default::default()})),
    ("21.103", ENCODE, solar_hot_water_status_tx, SolarHotWaterStatus{solar_load_sufficient: true, ..Default::default()}.encode(), [4]),
    ("21.103", DECODE, solar_hot_water_status_rx, SolarHotWaterStatus::decode(&[4]), Ok(SolarHotWaterStatus{solar_load_sufficient: true, ..Default::default()})),
    ("21.104", ENCODE, fuel_types_tx, FuelTypes{solid: true, ..Default::default()}.encode(), [4]),
    ("21.104", DECODE, fuel_types_rx, FuelTypes::decode(&[4]), Ok(FuelTypes{solid: true, ..Default::default()})),
    ("21.105", ENCODE, room_cooling_status_tx, RoomCoolingStatus{fault: true}.encode(), [1]),
    ("21.105", DECODE, room_cooling_status_rx, RoomCoolingStatus::decode(&[1]), Ok(RoomCoolingStatus{fault: true})),
    ("21.106", ENCODE, ventilation_status_tx, VentilationStatus{cooling: true, ..Default::default()}.encode(), [8]),
    ("21.106", DECODE, ventilation_status_rx, VentilationStatus::decode(&[8]), Ok(VentilationStatus{cooling: true, ..Default::default()})),
    ("22.100", ENCODE, hot_water_status_tx, HotWaterStatus{solar_support: true, ..Default::default()}.encode(), [0,64]),
    ("22.100", DECODE, hot_water_status_rx, HotWaterStatus::decode(&[0,64]), Ok(HotWaterStatus{solar_support: true, ..Default::default()})),
    ("22.101", ENCODE, room_climate_status_tx, RoomClimateStatus{frost_alarm: true, ..Default::default()}.encode(), [0x21,0]),
    ("22.101", DECODE, room_climate_status_rx, RoomClimateStatus::decode(&[0x21,0]), Ok(RoomClimateStatus{frost_alarm: true, ..Default::default()})),
    ("25.1000", ENCODE, repetition_counts_tx, Repetitions{busy:3,negative_ack:2}.encode(), Ok([0x32])),
    ("25.1000", DECODE, repetition_counts_rx, Repetitions::decode(&[0x32]), Ok(Repetitions{busy:3,negative_ack:2})),
    ("21.1010", ENCODE, channels_8_tx, encode_channels_8(0x81), [0x81]),
    ("21.1010", DECODE, channels_8_rx, decode_channels_8(&[0x81]), Ok(0x81)),
    ("22.1010", ENCODE, channels_16_tx, encode_channels_16(0x8102), [0x81,2]),
    ("22.1010", DECODE, channels_16_rx, decode_channels_16(&[0x81,2]), Ok(0x8102)),
    ("30.1010", ENCODE, channels_24_tx, encode_channels_24(0x810203), Ok([0x81,2,3])),
    ("30.1010", DECODE, channels_24_rx, decode_channels_24(&[0x81,2,3]), Ok(0x810203)),
    ("21.601", ENCODE, light_faults_tx, LightFaults{overheat:true,overcurrent:true,..Default::default()}.encode(), [68]),
    ("21.601", DECODE, light_faults_rx, LightFaults::decode(&[68]), Ok(LightFaults{overheat:true,overcurrent:true,..Default::default()})),
    ("237.600", ENCODE, gear_diagnostic_tx, GearDiagnostic{address:DaliAddress::Group(15),read_request:false,lamp_failure:true,ballast_failure:false,converter_error:true}.encode(), Ok([5,79])),
    ("237.600", DECODE, gear_diagnostic_rx, GearDiagnostic::decode(&[5,79]), Ok(GearDiagnostic{address:DaliAddress::Group(15),read_request:false,lamp_failure:true,ballast_failure:false,converter_error:true})),
    ("238.600", ENCODE, dali_diagnostic_tx, DaliDiagnostic{short_address:62,lamp_failure:true,ballast_failure:false}.encode(), Ok([126])),
    ("238.600", DECODE, dali_diagnostic_rx, DaliDiagnostic::decode(&[126]), Ok(DaliDiagnostic{short_address:62,lamp_failure:true,ballast_failure:false})),
    ("246.600", ENCODE, battery_info_tx, BatteryInfo{failed:true,duration_failure:true,fully_charged:false,charge:None}.encode(), Ok([3,255])),
    ("246.600", DECODE, battery_info_rx, BatteryInfo::decode(&[3,255]), Ok(BatteryInfo{failed:true,duration_failure:true,fully_charged:false,charge:None})),
    ("247.600", ENCODE, converter_test_info_tx, ConverterTestInfo{previous_start:DateTime::Fault,interval_hours:24,previous_seconds:257}.encode(), Ok([0,0,0,0,0,0,128,0,0,24,1,1])),
    ("247.600", DECODE, converter_test_info_rx, ConverterTestInfo::decode(&[0,0,0,0,0,0,128,0,0,24,1,1]), Ok(ConverterTestInfo{previous_start:DateTime::Fault,interval_hours:24,previous_seconds:257})),
    ("244.600", ENCODE, converter_status_tx, ConverterStatus{mode:ConverterMode::Rest,hardwired_inhibit:true,hardwired_switch:false,pending:[TestPending::Pending,TestPending::Clear,TestPending::Unknown],failure:Failure::Detected}.encode(), [65,146]),
    ("244.600", DECODE, converter_status_rx, ConverterStatus::decode(&[65,146]), Ok(ConverterStatus{mode:ConverterMode::Rest,hardwired_inhibit:true,hardwired_switch:false,pending:[TestPending::Pending,TestPending::Clear,TestPending::Unknown],failure:Failure::Detected})),
    ("245.600", ENCODE, converter_results_tx, ConverterResults{last:[LastTestResult::PassedOnTime,LastTestResult::StoppedManually,LastTestResult::FailedLate],started_by:[TestStart::Gateway,TestStart::Automatic,TestStart::Unknown],discharge_minutes:510,remaining_charge:None}.encode(), Ok([21,64,144,1,254,255])),
    ("245.600", DECODE, converter_results_rx, ConverterResults::decode(&[21,64,144,1,254,255]), Ok(ConverterResults{last:[LastTestResult::PassedOnTime,LastTestResult::StoppedManually,LastTestResult::FailedLate],started_by:[TestStart::Gateway,TestStart::Automatic,TestStart::Unknown],discharge_minutes:510,remaining_charge:None})),
    ("276.1200", ENCODE, erl_status_tx, ErlStatus{duty_cycles:[100,0,255],duty_cycle_alarms:[false,false,true],tic_alarm:true,..Default::default()}.encode(), [100,0,255,20]),
    ("276.1200", DECODE, erl_status_rx, ErlStatus::decode(&[100,0,255,20]), Ok(ErlStatus{duty_cycles:[100,0,255],duty_cycle_alarms:[false,false,true],tic_alarm:true,..Default::default()})),
    ("271.1200", ENCODE, tariff_day_tx, TariffDayItem{time:TimeOfDay{weekday:0,time:Time{hour:1,minute:2,second:3}},item:1,total:4,tariff:Tariff::UNAVAILABLE,profile:3,contact:ContactAction::Close,virtual_contacts:[true,false,false,false,false,false,true]}.encode(), Ok([1,2,3,20,0,15,65])),
    ("271.1200", DECODE, tariff_day_rx, TariffDayItem::decode(&[1,2,3,20,0,15,65]), Ok(TariffDayItem{time:TimeOfDay{weekday:0,time:Time{hour:1,minute:2,second:3}},item:1,total:4,tariff:Tariff::UNAVAILABLE,profile:3,contact:ContactAction::Close,virtual_contacts:[true,false,false,false,false,false,true]})),
    ("248.600", ENCODE, converter_fixed_tx, ConverterFixed::encode(ConverterInfo::Valid(ConverterFixed{minimum:None,maximum:Some(254),rated_duration_code:120,features:[false,true,false,false,false,false,false,true]})), Ok([1,255,254,120,130])),
    ("248.600", DECODE, converter_fixed_rx, ConverterFixed::decode(&[1,255,254,120,130]), Ok(ConverterInfo::Valid(ConverterFixed{minimum:None,maximum:Some(254),rated_duration_code:120,features:[false,true,false,false,false,false,false,true]}))),
    ("272.600", ENCODE, converter_runtime_tx, ConverterRuntime::encode(ConverterInfo::Valid(ConverterRuntime{emergency_hours:258,operating_hours:512,emergency_level:None,short_address:63})), Ok([1,1,2,2,0,255,63])),
    ("272.600", DECODE, converter_runtime_rx, ConverterRuntime::decode(&[1,1,2,2,0,255,63]), Ok(ConverterInfo::Valid(ConverterRuntime{emergency_hours:258,operating_hours:512,emergency_level:None,short_address:63}))),
    ("1.100", ENCODE, hvac_1_100_tx, hvac::HeatCool::Heating.encode(), [1]),
    ("1.100", DECODE, hvac_1_100_rx, hvac::HeatCool::decode(&[0]), Ok(hvac::HeatCool::Cooling)),
    ("20.100", ENCODE, hvac_20_100_tx, hvac::Fuel::Solid.encode(), [3]),
    ("20.100", DECODE, hvac_20_100_rx, hvac::Fuel::decode(&[0]), Ok(hvac::Fuel::Automatic)),
    ("20.101", ENCODE, hvac_20_101_tx, hvac::Burner::Modulating.encode(), [3]),
    ("20.101", DECODE, hvac_20_101_rx, hvac::Burner::decode(&[1]), Ok(hvac::Burner::SingleStage)),
    ("20.103", ENCODE, hvac_20_103_tx, hvac::HotWaterMode::OffFrostProtection.encode(), [4]),
    ("20.103", DECODE, hvac_20_103_rx, hvac::HotWaterMode::decode(&[0]), Ok(hvac::HotWaterMode::Auto)),
    ("20.104", ENCODE, hvac_20_104_tx, hvac::LoadPriority::Absolute.encode(), [2]),
    ("20.104", DECODE, hvac_20_104_rx, hvac::LoadPriority::decode(&[0]), Ok(hvac::LoadPriority::None)),
    ("20.105", ENCODE, hvac_20_105_tx, hvac::ControllerMode::NoDemand.encode(), [20]),
    ("20.105", DECODE, hvac_20_105_rx, hvac::ControllerMode::decode(&[0]), Ok(hvac::ControllerMode::Auto)),
    ("20.106", ENCODE, hvac_20_106_tx, hvac::EmergencyMode::Fire.encode(), [5]),
    ("20.106", DECODE, hvac_20_106_rx, hvac::EmergencyMode::decode(&[0]), Ok(hvac::EmergencyMode::Normal)),
    ("20.107", ENCODE, hvac_20_107_tx, hvac::Changeover::HeatingOnly.encode(), [2]),
    ("20.107", DECODE, hvac_20_107_rx, hvac::Changeover::decode(&[0]), Ok(hvac::Changeover::Automatic)),
    ("20.108", ENCODE, hvac_20_108_tx, hvac::ValveMode::Changeover.encode(), [5]),
    ("20.108", DECODE, hvac_20_108_rx, hvac::ValveMode::decode(&[1]), Ok(hvac::ValveMode::HeatStageA)),
    ("20.109", ENCODE, hvac_20_109_tx, hvac::DamperMode::ExtractAir.encode(), [4]),
    ("20.109", DECODE, hvac_20_109_rx, hvac::DamperMode::decode(&[1]), Ok(hvac::DamperMode::FreshAir)),
    ("20.110", ENCODE, hvac_20_110_tx, hvac::HeaterMode::StageBProportional.encode(), [3]),
    ("20.110", DECODE, hvac_20_110_rx, hvac::HeaterMode::decode(&[1]), Ok(hvac::HeaterMode::StageAOnOff)),
    ("20.111", ENCODE, hvac_20_111_tx, hvac::FanMode::Intermittent.encode(), [2]),
    ("20.111", DECODE, hvac_20_111_rx, hvac::FanMode::decode(&[0]), Ok(hvac::FanMode::Stopped)),
    ("20.112", ENCODE, hvac_20_112_tx, hvac::ControllerRole::Slave.encode(), [2]),
    ("20.112", DECODE, hvac_20_112_rx, hvac::ControllerRole::decode(&[0]), Ok(hvac::ControllerRole::Autonomous)),
    ("20.113", ENCODE, hvac_20_113_tx, hvac::RoomSetpoint::BuildingProtection.encode(), [2]),
    ("20.113", DECODE, hvac_20_113_rx, hvac::RoomSetpoint::decode(&[0]), Ok(hvac::RoomSetpoint::Normal)),
    ("20.114", ENCODE, hvac_20_114_tx, hvac::MeterDevice::Void.encode(), [255]),
    ("20.114", DECODE, hvac_20_114_rx, hvac::MeterDevice::decode(&[0]), Ok(hvac::MeterDevice::Other)),
    ("20.115", ENCODE, hvac_20_115_tx, hvac::HumidityMode::Dehumidify.encode(), [2]),
    ("20.115", DECODE, hvac_20_115_rx, hvac::HumidityMode::decode(&[0]), Ok(hvac::HumidityMode::Inactive)),
    ("20.120", ENCODE, hvac_20_120_tx, hvac::DamperType::VariableAirVolume.encode(), [2]),
    ("20.120", DECODE, hvac_20_120_rx, hvac::DamperType::decode(&[1]), Ok(hvac::DamperType::AirDamper)),
    ("20.121", ENCODE, hvac_20_121_tx, hvac::BackupMode::RetainLastState.encode(), [1]),
    ("20.121", DECODE, hvac_20_121_rx, hvac::BackupMode::decode(&[0]), Ok(hvac::BackupMode::BackupValue)),
    ("20.122", ENCODE, hvac_20_122_tx, hvac::Synchronization::OpenOnce.encode(), [2]),
    ("20.122", DECODE, hvac_20_122_rx, hvac::Synchronization::decode(&[0]), Ok(hvac::Synchronization::Unchanged)),
    ("23.102", ENCODE, hvac_23_102_tx, hvac::PushButtonAction::ProtectionAuto.encode(), [3]),
    ("23.102", DECODE, hvac_23_102_rx, hvac::PushButtonAction::decode(&[0]), Ok(hvac::PushButtonAction::ComfortEconomy)),
    ("31.101", ENCODE, hvac_31_101_tx, hvac::PushButtonParameter::ComfortStandby.encode(), [7]),
    ("31.101", DECODE, hvac_31_101_rx, hvac::PushButtonParameter::decode(&[0]), Ok(hvac::PushButtonParameter::ComfortEconomy)),
    ("7.600", ENCODE, colour_kelvin_tx, encode_colour_kelvin(3000), [11,184]),
    ("7.600", DECODE, colour_kelvin_rx, decode_colour_kelvin(&[11,184]), Ok(3000)),
    ("12.1200", ENCODE, volume_litres_tx, encode_volume_litres(258), [0,0,1,2]),
    ("12.1200", DECODE, volume_litres_rx, decode_volume_litres(&[0,0,1,2]), Ok(258)),
    ("12.1201", ENCODE, volume_cubic_metres_tx, encode_volume_cubic_metres(65537), [0,1,0,1]),
    ("12.1201", DECODE, volume_cubic_metres_rx, decode_volume_cubic_metres(&[0,1,0,1]), Ok(65537)),
    ("13.1200", ENCODE, delta_litres_tx, encode_delta_litres(-2), [255,255,255,254]),
    ("13.1200", DECODE, delta_litres_rx, decode_delta_litres(&[255,255,255,254]), Ok(-2)),
    ("13.1201", ENCODE, delta_cubic_metres_tx, encode_delta_cubic_metres(-32768), [255,255,128,0]),
    ("13.1201", DECODE, delta_cubic_metres_rx, decode_delta_cubic_metres(&[255,255,128,0]), Ok(-32768)),
    ("14.1200", ENCODE, flow_m3h_tx, Float32Dpt::from_dpt("14.1200").unwrap().encode(1.0), Ok([63,128,0,0])),
    ("14.1200", DECODE, flow_m3h_rx, Float32Dpt::from_dpt("14.1200").unwrap().decode(&[63,128,0,0]), Ok(1.0)),
    ("14.1201", ENCODE, flow_ls_tx, Float32Dpt::from_dpt("14.1201").unwrap().encode(1.0), Ok([63,128,0,0])),
    ("14.1201", DECODE, flow_ls_rx, Float32Dpt::from_dpt("14.1201").unwrap().decode(&[63,128,0,0]), Ok(1.0)),
    ("242.600", ENCODE, xy_colour_tx, XyColor{coordinates:Some((0.0,1.0)),brightness:Some(128)}.encode(), Ok([0,0,255,255,128,3])),
    ("242.600", DECODE, xy_colour_rx, XyColor::decode(&[0,0,255,255,128,3]), Ok(XyColor{coordinates:Some((0.0,1.0)),brightness:Some(128)})),
    ("243.600", ENCODE, xy_transition_tx, XyColor{coordinates:Some((0.0,1.0)),brightness:Some(128)}.encode_transition(42), Ok([0,42,255,255,0,0,128,3])),
    ("243.600", DECODE, xy_transition_rx, XyColor::decode_transition(&[0,42,255,255,0,0,128,3]), Ok((42,XyColor{coordinates:Some((0.0,1.0)),brightness:Some(128)}))),
    ("249.600", ENCODE, white_transition_tx, WhiteTransition{ticks_100ms:Some(42),kelvin:Some(3000),brightness:None}.encode(), [0,42,11,184,0,6]),
    ("249.600", DECODE, white_transition_rx, WhiteTransition::decode(&[0,42,11,184,0,6]), Ok(WhiteTransition{ticks_100ms:Some(42),kelvin:Some(3000),brightness:None})),
    ("250.600", ENCODE, white_control_tx, WhiteControl{brightness:Some(Dimming::Stop),..Default::default()}.encode(), [0,0,1]),
    ("250.600", DECODE, white_control_rx, WhiteControl::decode(&[0,0,1]), Ok(WhiteControl{brightness:Some(Dimming::Stop),..Default::default()})),
    ("252.600", ENCODE, rgbw_control_tx, RgbwControl{red:Some(Dimming::Stop),white:Some(Dimming::Stop),..Default::default()}.encode(), [0,0,0,0,9]),
    ("252.600", DECODE, rgbw_control_rx, RgbwControl::decode(&[0,0,0,0,9]), Ok(RgbwControl{red:Some(Dimming::Stop),white:Some(Dimming::Stop),..Default::default()})),
    ("253.600", ENCODE, xy_control_tx, XyControl{wavelength:Some(Dimming::Stop),..Default::default()}.encode(), [0,0,0,2]),
    ("253.600", DECODE, xy_control_rx, XyControl::decode(&[0,0,0,2]), Ok(XyControl{wavelength:Some(Dimming::Stop),..Default::default()})),
    ("254.600", ENCODE, rgb_steps_tx, encode_rgb_steps([Dimming::Stop;3]), [0;3]),
    ("254.600", DECODE, rgb_steps_rx, decode_rgb_steps(&[8,0,8]), Ok([Dimming::Stop;3])),
    ("273.001", ENCODE, forecast_temp_tx, WeatherDpt::from_dpt("273.001").unwrap().encode(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()}), Ok([3,0,0,0,0,200,0,100])),
    ("273.001", DECODE, forecast_temp_rx, WeatherDpt::from_dpt("273.001").unwrap().decode(&[3,0,0,0,0,200,0,100]), Ok(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()})),
    ("273.002", ENCODE, forecast_wind_tx, WeatherDpt::from_dpt("273.002").unwrap().encode(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()}), Ok([3,0,0,0,0,200,0,100])),
    ("273.002", DECODE, forecast_wind_rx, WeatherDpt::from_dpt("273.002").unwrap().decode(&[3,0,0,0,0,200,0,100]), Ok(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()})),
    ("273.003", ENCODE, forecast_relative_humidity_tx, WeatherDpt::from_dpt("273.003").unwrap().encode(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()}), Ok([3,0,0,0,0,200,0,100])),
    ("273.003", DECODE, forecast_relative_humidity_rx, WeatherDpt::from_dpt("273.003").unwrap().decode(&[3,0,0,0,0,200,0,100]), Ok(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()})),
    ("273.004", ENCODE, forecast_absolute_humidity_tx, WeatherDpt::from_dpt("273.004").unwrap().encode(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()}), Ok([3,0,0,0,0,200,0,100])),
    ("273.004", DECODE, forecast_absolute_humidity_rx, WeatherDpt::from_dpt("273.004").unwrap().decode(&[3,0,0,0,0,200,0,100]), Ok(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()})),
    ("273.005", ENCODE, forecast_co2_tx, WeatherDpt::from_dpt("273.005").unwrap().encode(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()}), Ok([3,0,0,0,0,200,0,100])),
    ("273.005", DECODE, forecast_co2_rx, WeatherDpt::from_dpt("273.005").unwrap().decode(&[3,0,0,0,0,200,0,100]), Ok(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()})),
    ("273.006", ENCODE, forecast_pollutants_tx, WeatherDpt::from_dpt("273.006").unwrap().encode(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()}), Ok([3,0,0,0,0,200,0,100])),
    ("273.006", DECODE, forecast_pollutants_rx, WeatherDpt::from_dpt("273.006").unwrap().decode(&[3,0,0,0,0,200,0,100]), Ok(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()})),
    ("273.007", ENCODE, forecast_sun_tx, WeatherDpt::from_dpt("273.007").unwrap().encode(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()}), Ok([3,0,0,0,0,200,0,100])),
    ("273.007", DECODE, forecast_sun_rx, WeatherDpt::from_dpt("273.007").unwrap().decode(&[3,0,0,0,0,200,0,100]), Ok(Forecast{maximum:Some(2.0),minimum:Some(1.0),..Default::default()})),
    ("274.001", ENCODE, forecast_direction_tx, encode_wind_forecast(Forecast{maximum:Some(360.0),minimum:Some(0.0),..Default::default()}), Ok([3,0,0,0,255,0])),
    ("274.001", DECODE, forecast_direction_rx, decode_wind_forecast(&[3,0,0,0,255,0]), Ok(Forecast{maximum:Some(360.0),minimum:Some(0.0),..Default::default()})),
    ("1.1200", ENCODE, meter_role_tx, MeterRole::Producer.encode(), [1]),
    ("1.1200", DECODE, meter_role_rx, MeterRole::decode(&[1]), Ok(MeterRole::Producer)),
    ("1.1201", ENCODE, energy_direction_tx, EnergyDirection::Negative.encode(), [1]),
    ("1.1201", DECODE, energy_direction_rx, EnergyDirection::decode(&[1]), Ok(EnergyDirection::Negative)),
    ("20.600", ENCODE, light_lock_behavior_tx, LightLockBehavior::BeforeLock.encode(), [6]),
    ("20.600", DECODE, light_lock_behavior_rx, LightLockBehavior::decode(&[6]), Ok(LightLockBehavior::BeforeLock)),
    ("20.601", ENCODE, bus_power_behavior_tx, BusPowerBehavior::BeforePowerDown.encode(), [4]),
    ("20.601", DECODE, bus_power_behavior_rx, BusPowerBehavior::decode(&[4]), Ok(BusPowerBehavior::BeforePowerDown)),
    ("20.603", ENCODE, blink_mode_tx, BlinkMode::WithAcknowledgement.encode(), [2]),
    ("20.603", DECODE, blink_mode_rx, BlinkMode::decode(&[2]), Ok(BlinkMode::WithAcknowledgement)),
    ("20.604", ENCODE, light_control_mode_tx, LightControlMode::Manual.encode(), [1]),
    ("20.604", DECODE, light_control_mode_rx, LightControlMode::decode(&[1]), Ok(LightControlMode::Manual)),
    ("20.605", ENCODE, switch_input_mode_tx, SwitchInputMode::TwoButtons.encode(), [2]),
    ("20.605", DECODE, switch_input_mode_rx, SwitchInputMode::decode(&[2]), Ok(SwitchInputMode::TwoButtons)),
    ("20.606", ENCODE, switch_button_action_tx, SwitchButtonAction::InvertFeedback.encode(), [3]),
    ("20.606", DECODE, switch_button_action_rx, SwitchButtonAction::decode(&[3]), Ok(SwitchButtonAction::InvertFeedback)),
    ("20.607", ENCODE, dimming_input_mode_tx, DimmingInputMode::TwoButtons.encode(), [4]),
    ("20.607", DECODE, dimming_input_mode_rx, DimmingInputMode::decode(&[4]), Ok(DimmingInputMode::TwoButtons)),
    ("20.608", ENCODE, switch_on_mode_tx, SwitchOnMode::LastAbsoluteSetpoint.encode(), [2]),
    ("20.608", DECODE, switch_on_mode_rx, SwitchOnMode::decode(&[2]), Ok(SwitchOnMode::LastAbsoluteSetpoint)),
    ("20.609", ENCODE, load_type_set_tx, LoadTypeSet::LedTrailing.encode(), [8]),
    ("20.609", DECODE, load_type_set_rx, LoadTypeSet::decode(&[8]), Ok(LoadTypeSet::LedTrailing)),
    ("20.610", ENCODE, detected_load_tx, DetectedLoad::LedTrailing.encode(), [8]),
    ("20.610", DECODE, detected_load_rx, DetectedLoad::decode(&[8]), Ok(DetectedLoad::LedTrailing)),
    ("20.611", ENCODE, converter_test_control_tx, ConverterTestControl::ResetDurationTestDone.encode(), [6]),
    ("20.611", DECODE, converter_test_control_rx, ConverterTestControl::decode(&[6]), Ok(ConverterTestControl::ResetDurationTestDone)),
    ("20.612", ENCODE, converter_control_tx, ConverterControl::ResetLampTime.encode(), [4]),
    ("20.612", DECODE, converter_control_rx, ConverterControl::decode(&[4]), Ok(ConverterControl::ResetLampTime)),
    ("20.613", ENCODE, converter_data_request_tx, ConverterDataRequest::FixedConverterInfo.encode(), [8]),
    ("20.613", DECODE, converter_data_request_rx, ConverterDataRequest::decode(&[8]), Ok(ConverterDataRequest::FixedConverterInfo)),
    ("20.801", ENCODE, blind_exception_behavior_tx, BlindExceptionBehavior::Stop.encode(), [4]),
    ("20.801", DECODE, blind_exception_behavior_rx, BlindExceptionBehavior::decode(&[4]), Ok(BlindExceptionBehavior::Stop)),
    ("20.802", ENCODE, blind_lock_behavior_tx, BlindLockBehavior::BeforeLock.encode(), [6]),
    ("20.802", DECODE, blind_lock_behavior_rx, BlindLockBehavior::decode(&[6]), Ok(BlindLockBehavior::BeforeLock)),
    ("20.803", ENCODE, blind_input_mode_tx, BlindInputMode::TwoButtons.encode(), [4]),
    ("20.803", DECODE, blind_input_mode_rx, BlindInputMode::decode(&[4]), Ok(BlindInputMode::TwoButtons)),
    ("20.804", ENCODE, blind_control_mode_tx, BlindControlMode::Manual.encode(), [1]),
    ("20.804", DECODE, blind_control_mode_rx, BlindControlMode::decode(&[1]), Ok(BlindControlMode::Manual)),
    ("20.1202", ENCODE, gas_measurement_condition_tx, GasMeasurementCondition::MeasurementConditions.encode(), [3]),
    ("20.1202", DECODE, gas_measurement_condition_rx, GasMeasurementCondition::decode(&[3]), Ok(GasMeasurementCondition::MeasurementConditions)),
    ("20.1203", ENCODE, breaker_status_tx, BreakerStatus::OpenOverheatBelowMaxCurrent.encode(), [6]),
    ("20.1203", DECODE, breaker_status_rx, BreakerStatus::decode(&[6]), Ok(BreakerStatus::OpenOverheatBelowMaxCurrent)),
    ("20.1204", ENCODE, euridis_status_tx, EuridisStatus::ActiveSecured.encode(), [2]),
    ("20.1204", DECODE, euridis_status_rx, EuridisStatus::decode(&[2]), Ok(EuridisStatus::ActiveSecured)),
    ("20.1206", ENCODE, peak_notice_tx, PeakNotice::NoticeThree.encode(), [3]),
    ("20.1206", DECODE, peak_notice_rx, PeakNotice::decode(&[3]), Ok(PeakNotice::NoticeThree)),
    ("20.1207", ENCODE, peak_event_tx, PeakEvent::EventThree.encode(), [3]),
    ("20.1207", DECODE, peak_event_rx, PeakEvent::decode(&[3]), Ok(PeakEvent::EventThree)),
    ("20.1208", ENCODE, tic_type_tx, TicType::Standard.encode(), [1]),
    ("20.1208", DECODE, tic_type_rx, TicType::decode(&[1]), Ok(TicType::Standard)),
    ("20.1209", ENCODE, tic_channel_tx, TicChannel::StandardThreePhase.encode(), [4]),
    ("20.1209", DECODE, tic_channel_rx, TicChannel::decode(&[4]), Ok(TicChannel::StandardThreePhase)),
    ("20.1200", ENCODE, breaker_valve_state_tx, BreakerValveState::Invalid.encode(), [255]),
    ("20.1200", DECODE, breaker_valve_state_rx, BreakerValveState::decode(&[255]), Ok(BreakerValveState::Invalid)),
    ("265.1200", ENCODE, meter_role_dated_tx, MeterRole::Producer.encode_dated(DateTime::Fault), Ok([0,0,0,0,0,0,128,0,1])),
    ("265.1200", DECODE, meter_role_dated_rx, MeterRole::decode_dated(&[0,0,0,0,0,0,128,0,1]), Ok((DateTime::Fault,MeterRole::Producer))),
    ("265.1201", ENCODE, energy_direction_dated_tx, EnergyDirection::Negative.encode_dated(DateTime::Fault), Ok([0,0,0,0,0,0,128,0,1])),
    ("265.1201", DECODE, energy_direction_dated_rx, EnergyDirection::decode_dated(&[0,0,0,0,0,0,128,0,1]), Ok((DateTime::Fault,EnergyDirection::Negative))),
    ("268.1203", ENCODE, breaker_status_dated_tx, BreakerStatus::OpenOverload.encode_dated(DateTime::Fault), Ok([0,0,0,0,0,0,128,0,1])),
    ("268.1203", DECODE, breaker_status_dated_rx, BreakerStatus::decode_dated(&[0,0,0,0,0,0,128,0,1]), Ok((DateTime::Fault,BreakerStatus::OpenOverload))),
    ("268.1204", ENCODE, euridis_status_dated_tx, EuridisStatus::ActiveSecured.encode_dated(DateTime::Fault), Ok([0,0,0,0,0,0,128,0,2])),
    ("268.1204", DECODE, euridis_status_dated_rx, EuridisStatus::decode_dated(&[0,0,0,0,0,0,128,0,2]), Ok((DateTime::Fault,EuridisStatus::ActiveSecured))),
    ("268.1206", ENCODE, peak_notice_dated_tx, PeakNotice::NoticeThree.encode_dated(DateTime::Fault), Ok([0,0,0,0,0,0,128,0,3])),
    ("268.1206", DECODE, peak_notice_dated_rx, PeakNotice::decode_dated(&[0,0,0,0,0,0,128,0,3]), Ok((DateTime::Fault,PeakNotice::NoticeThree))),
    ("20.1205", ENCODE, plc_status_tx, PlcTechnology::G3.encode(PlcStatus::Associated), Ok([1])),
    ("20.1205", DECODE, plc_status_rx, PlcTechnology::Sfsk.decode(&[2]), Ok(PlcStatus::Registered)),
    ("268.1205", ENCODE, plc_status_dated_tx, PlcTechnology::Sfsk.encode_dated(DateTime::Fault,PlcStatus::Registered), Ok([0,0,0,0,0,0,128,0,2])),
    ("268.1205", DECODE, plc_status_dated_rx, PlcTechnology::Sfsk.decode_dated(&[0,0,0,0,0,0,128,0,2]), Ok((DateTime::Fault,PlcStatus::Registered))),
    ("21.1200", ENCODE, contact_bank_tx, encode_contacts([true,false,false,false,false,false,false,true]), [129]),
    ("21.1200", DECODE, contact_bank_rx, decode_contacts(&[129]), Ok([true,false,false,false,false,false,false,true])),
    ("21.1201", ENCODE, phase_presence_tx, encode_phase_presence([true,false,true]), [5]),
    ("21.1201", DECODE, phase_presence_rx, decode_phase_presence(&[5]), Ok([true,false,true])),
    ("20.602", ENCODE, fade_tenths_tx, encode_fade_tenths(57), Ok([7])),
    ("20.602", DECODE, fade_tenths_rx, decode_fade_tenths(&[15]), Ok(905)),
    ("240.800", ENCODE, blind_position_tx, CombinedPosition{height:None,slats:Some(255)}.encode(), [0,255,2]),
    ("240.800", DECODE, blind_position_rx, CombinedPosition::decode(&[99,255,2]), Ok(CombinedPosition{height:None,slats:Some(255)})),
    ("241.800", ENCODE, blind_status_tx, BlindStatus{weather_alarm:true,failure:true,..Default::default()}.encode(), [0,0,4,64]),
    ("241.800", DECODE, blind_status_rx, BlindStatus::decode(&[22,33,4,64]), Ok(BlindStatus{weather_alarm:true,failure:true,..Default::default()})),
    ("232.600", ENCODE, rgb_tx, Rgb{red:1,green:128,blue:255}.encode(), [1,128,255]),
    ("232.600", DECODE, rgb_rx, Rgb::decode(&[1,128,255]), Ok(Rgb{red:1,green:128,blue:255})),
    ("251.600", ENCODE, rgbw_tx, Rgbw{red:Some(64),white:Some(42),..Default::default()}.encode(), [64,0,0,42,0,9]),
    ("251.600", DECODE, rgbw_rx, Rgbw::decode(&[64,128,255,42,0,9]), Ok(Rgbw{red:Some(64),white:Some(42),..Default::default()})),
    ("257.1200", ENCODE, current_phases_tx, PhaseQuantity::Current.encode([1.0,-2.0,0.5]), Ok([63,128,0,0,192,0,0,0,63,0,0,0])),
    ("257.1200", DECODE, current_phases_rx, PhaseQuantity::Current.decode(&[63,128,0,0,192,0,0,0,63,0,0,0]), Ok([1.0,-2.0,0.5])),
    ("270.1200", ENCODE, current_dated_phases_tx, PhaseQuantity::Current.encode_dated(DateTime::Fault,[1.0,-2.0,0.5]), Ok([0,0,0,0,0,0,128,0,63,128,0,0,192,0,0,0,63,0,0,0])),
    ("270.1200", DECODE, current_dated_phases_rx, PhaseQuantity::Current.decode_dated(&[0,0,0,0,0,0,128,0,63,128,0,0,192,0,0,0,63,0,0,0]), Ok((DateTime::Fault,[1.0,-2.0,0.5]))),
    ("257.1201", ENCODE, potential_phases_tx, PhaseQuantity::Potential.encode([1.0,-2.0,0.5]), Ok([63,128,0,0,192,0,0,0,63,0,0,0])),
    ("257.1201", DECODE, potential_phases_rx, PhaseQuantity::Potential.decode(&[63,128,0,0,192,0,0,0,63,0,0,0]), Ok([1.0,-2.0,0.5])),
    ("270.1201", ENCODE, potential_dated_phases_tx, PhaseQuantity::Potential.encode_dated(DateTime::Fault,[1.0,-2.0,0.5]), Ok([0,0,0,0,0,0,128,0,63,128,0,0,192,0,0,0,63,0,0,0])),
    ("270.1201", DECODE, potential_dated_phases_rx, PhaseQuantity::Potential.decode_dated(&[0,0,0,0,0,0,128,0,63,128,0,0,192,0,0,0,63,0,0,0]), Ok((DateTime::Fault,[1.0,-2.0,0.5]))),
    ("257.1202", ENCODE, apparent_power_phases_tx, PhaseQuantity::ApparentPower.encode([1.0,-2.0,0.5]), Ok([63,128,0,0,192,0,0,0,63,0,0,0])),
    ("257.1202", DECODE, apparent_power_phases_rx, PhaseQuantity::ApparentPower.decode(&[63,128,0,0,192,0,0,0,63,0,0,0]), Ok([1.0,-2.0,0.5])),
    ("270.1202", ENCODE, apparent_power_dated_phases_tx, PhaseQuantity::ApparentPower.encode_dated(DateTime::Fault,[1.0,-2.0,0.5]), Ok([0,0,0,0,0,0,128,0,63,128,0,0,192,0,0,0,63,0,0,0])),
    ("270.1202", DECODE, apparent_power_dated_phases_rx, PhaseQuantity::ApparentPower.decode_dated(&[0,0,0,0,0,0,128,0,63,128,0,0,192,0,0,0,63,0,0,0]), Ok((DateTime::Fault,[1.0,-2.0,0.5]))),
    ("207.600", ENCODE, lighting_status_tx, LightingStatus{level:None,locked:false,forced:false,night_mode:false,staircase:false,dimming:false,local_override:true,failure:true}.encode(LightingActuator::Switch), Ok([0,192])),
    ("207.600", DECODE, lighting_status_rx, LightingStatus::decode(LightingActuator::Switch,&[127,192]), Ok(LightingStatus{level:None,locked:false,forced:false,night_mode:false,staircase:false,dimming:false,local_override:true,failure:true})),
    ("269.1200", ENCODE, dated_energy_tx, DatedTariffEnergy{timestamp:knx::text_time::DateTime::Fault, register:TariffEnergy{watt_hours:Some(300),tariff:None}}.encode(), Ok([0,0,0,0,0,0,128,0,0,0,1,44,0,1])),
    ("269.1200", DECODE, dated_energy_rx, DatedTariffEnergy::decode(&[0,0,0,0,0,0,128,0,0,0,1,44,0,1]), Ok(DatedTariffEnergy{timestamp:knx::text_time::DateTime::Fault, register:TariffEnergy{watt_hours:Some(300),tariff:None}})),
    (
        "1.002",
        ENCODE,
        binary_2_tx,
        BinaryDpt::from_dpt("1.002").unwrap().encode("true"),
        Ok([1])
    ),
    (
        "1.002",
        DECODE,
        binary_2_rx,
        BinaryDpt::from_dpt("1.002").unwrap().decode(&[1]),
        Ok("true")
    ),
    (
        "1.003",
        ENCODE,
        binary_3_tx,
        BinaryDpt::from_dpt("1.003").unwrap().encode("enable"),
        Ok([1])
    ),
    (
        "1.003",
        DECODE,
        binary_3_rx,
        BinaryDpt::from_dpt("1.003").unwrap().decode(&[1]),
        Ok("enable")
    ),
    (
        "1.004",
        ENCODE,
        binary_4_tx,
        BinaryDpt::from_dpt("1.004").unwrap().encode("ramp"),
        Ok([1])
    ),
    (
        "1.004",
        DECODE,
        binary_4_rx,
        BinaryDpt::from_dpt("1.004").unwrap().decode(&[1]),
        Ok("ramp")
    ),
    (
        "1.005",
        ENCODE,
        binary_5_tx,
        BinaryDpt::from_dpt("1.005").unwrap().encode("alarm"),
        Ok([1])
    ),
    (
        "1.005",
        DECODE,
        binary_5_rx,
        BinaryDpt::from_dpt("1.005").unwrap().decode(&[1]),
        Ok("alarm")
    ),
    (
        "1.006",
        ENCODE,
        binary_6_tx,
        BinaryDpt::from_dpt("1.006").unwrap().encode("high"),
        Ok([1])
    ),
    (
        "1.006",
        DECODE,
        binary_6_rx,
        BinaryDpt::from_dpt("1.006").unwrap().decode(&[1]),
        Ok("high")
    ),
    (
        "1.007",
        ENCODE,
        binary_7_tx,
        BinaryDpt::from_dpt("1.007").unwrap().encode("increase"),
        Ok([1])
    ),
    (
        "1.007",
        DECODE,
        binary_7_rx,
        BinaryDpt::from_dpt("1.007").unwrap().decode(&[1]),
        Ok("increase")
    ),
    (
        "1.008",
        ENCODE,
        binary_8_tx,
        BinaryDpt::from_dpt("1.008").unwrap().encode("down"),
        Ok([1])
    ),
    (
        "1.008",
        DECODE,
        binary_8_rx,
        BinaryDpt::from_dpt("1.008").unwrap().decode(&[1]),
        Ok("down")
    ),
    (
        "1.009",
        ENCODE,
        binary_9_tx,
        BinaryDpt::from_dpt("1.009").unwrap().encode("close"),
        Ok([1])
    ),
    (
        "1.009",
        DECODE,
        binary_9_rx,
        BinaryDpt::from_dpt("1.009").unwrap().decode(&[1]),
        Ok("close")
    ),
    (
        "1.010",
        ENCODE,
        binary_10_tx,
        BinaryDpt::from_dpt("1.010").unwrap().encode("start"),
        Ok([1])
    ),
    (
        "1.010",
        DECODE,
        binary_10_rx,
        BinaryDpt::from_dpt("1.010").unwrap().decode(&[1]),
        Ok("start")
    ),
    (
        "1.011",
        ENCODE,
        binary_11_tx,
        BinaryDpt::from_dpt("1.011").unwrap().encode("active"),
        Ok([1])
    ),
    (
        "1.011",
        DECODE,
        binary_11_rx,
        BinaryDpt::from_dpt("1.011").unwrap().decode(&[1]),
        Ok("active")
    ),
    (
        "1.012",
        ENCODE,
        binary_12_tx,
        BinaryDpt::from_dpt("1.012").unwrap().encode("inverted"),
        Ok([1])
    ),
    (
        "1.012",
        DECODE,
        binary_12_rx,
        BinaryDpt::from_dpt("1.012").unwrap().decode(&[1]),
        Ok("inverted")
    ),
    (
        "1.013",
        ENCODE,
        binary_13_tx,
        BinaryDpt::from_dpt("1.013").unwrap().encode("cyclic"),
        Ok([1])
    ),
    (
        "1.013",
        DECODE,
        binary_13_rx,
        BinaryDpt::from_dpt("1.013").unwrap().decode(&[1]),
        Ok("cyclic")
    ),
    (
        "1.014",
        ENCODE,
        binary_14_tx,
        BinaryDpt::from_dpt("1.014").unwrap().encode("calculated"),
        Ok([1])
    ),
    (
        "1.014",
        DECODE,
        binary_14_rx,
        BinaryDpt::from_dpt("1.014").unwrap().decode(&[1]),
        Ok("calculated")
    ),
    (
        "1.015",
        ENCODE,
        binary_15_tx,
        BinaryDpt::from_dpt("1.015").unwrap().encode("reset"),
        Ok([1])
    ),
    (
        "1.015",
        DECODE,
        binary_15_rx,
        BinaryDpt::from_dpt("1.015").unwrap().decode(&[1]),
        Ok("reset")
    ),
    (
        "1.016",
        ENCODE,
        binary_16_tx,
        BinaryDpt::from_dpt("1.016").unwrap().encode("acknowledge"),
        Ok([1])
    ),
    (
        "1.016",
        DECODE,
        binary_16_rx,
        BinaryDpt::from_dpt("1.016").unwrap().decode(&[1]),
        Ok("acknowledge")
    ),
    (
        "1.017",
        ENCODE,
        binary_17_tx,
        BinaryDpt::from_dpt("1.017").unwrap().encode("trigger"),
        Ok([0])
    ),
    (
        "1.017",
        DECODE,
        binary_17_rx,
        BinaryDpt::from_dpt("1.017").unwrap().decode(&[1]),
        Ok("trigger")
    ),
    (
        "1.018",
        ENCODE,
        binary_18_tx,
        BinaryDpt::from_dpt("1.018").unwrap().encode("occupied"),
        Ok([1])
    ),
    (
        "1.018",
        DECODE,
        binary_18_rx,
        BinaryDpt::from_dpt("1.018").unwrap().decode(&[1]),
        Ok("occupied")
    ),
    (
        "1.019",
        ENCODE,
        binary_19_tx,
        BinaryDpt::from_dpt("1.019").unwrap().encode("open"),
        Ok([1])
    ),
    (
        "1.019",
        DECODE,
        binary_19_rx,
        BinaryDpt::from_dpt("1.019").unwrap().decode(&[1]),
        Ok("open")
    ),
    (
        "1.021",
        ENCODE,
        binary_21_tx,
        BinaryDpt::from_dpt("1.021").unwrap().encode("and"),
        Ok([1])
    ),
    (
        "1.021",
        DECODE,
        binary_21_rx,
        BinaryDpt::from_dpt("1.021").unwrap().decode(&[1]),
        Ok("and")
    ),
    (
        "1.022",
        ENCODE,
        binary_22_tx,
        BinaryDpt::from_dpt("1.022").unwrap().encode("scene-2"),
        Ok([1])
    ),
    (
        "1.022",
        DECODE,
        binary_22_rx,
        BinaryDpt::from_dpt("1.022").unwrap().decode(&[1]),
        Ok("scene-2")
    ),
    (
        "1.023",
        ENCODE,
        binary_23_tx,
        BinaryDpt::from_dpt("1.023").unwrap().encode("blind"),
        Ok([1])
    ),
    (
        "1.023",
        DECODE,
        binary_23_rx,
        BinaryDpt::from_dpt("1.023").unwrap().decode(&[1]),
        Ok("blind")
    ),
    (
        "1.024",
        ENCODE,
        binary_24_tx,
        BinaryDpt::from_dpt("1.024").unwrap().encode("night"),
        Ok([1])
    ),
    (
        "1.024",
        DECODE,
        binary_24_rx,
        BinaryDpt::from_dpt("1.024").unwrap().decode(&[1]),
        Ok("night")
    ),
    (
        "7.001",
        ENCODE,
        pulse_count_u16_tx,
        encode_pulse_count_u16(258),
        [1, 2]
    ),
    (
        "7.001",
        DECODE,
        pulse_count_u16_rx,
        decode_pulse_count_u16(&[1, 2]),
        Ok(258)
    ),
    (
        "7.010",
        ENCODE,
        property_data_type_tx,
        encode_property_data_type(258),
        [1, 2]
    ),
    (
        "7.010",
        DECODE,
        property_data_type_rx,
        decode_property_data_type(&[1, 2]),
        Ok(258)
    ),
    ("7.011", ENCODE, length_mm_tx, encode_length_mm(258), [1, 2]),
    (
        "7.011",
        DECODE,
        length_mm_rx,
        decode_length_mm(&[1, 2]),
        Ok(258)
    ),
    (
        "7.013",
        ENCODE,
        parameter_lux_tx,
        encode_parameter_lux(258),
        [1, 2]
    ),
    (
        "7.013",
        DECODE,
        parameter_lux_rx,
        decode_parameter_lux(&[1, 2]),
        Ok(258)
    ),
    (
        "8.001",
        ENCODE,
        pulse_count_i16_tx,
        encode_pulse_count_i16(-258),
        [254, 254]
    ),
    (
        "8.001",
        DECODE,
        pulse_count_i16_rx,
        decode_pulse_count_i16(&[254, 254]),
        Ok(-258)
    ),
    (
        "8.011",
        ENCODE,
        rotation_degrees_tx,
        encode_rotation_degrees(-258),
        [254, 254]
    ),
    (
        "8.011",
        DECODE,
        rotation_degrees_rx,
        decode_rotation_degrees(&[254, 254]),
        Ok(-258)
    ),
    (
        "8.012",
        ENCODE,
        altitude_m_tx,
        encode_altitude_m(-258),
        [254, 254]
    ),
    (
        "8.012",
        DECODE,
        altitude_m_rx,
        decode_altitude_m(&[254, 254]),
        Ok(-258)
    ),
    (
        "12.001",
        ENCODE,
        pulse_count_u32_tx,
        encode_pulse_count_u32(258),
        [0, 0, 1, 2]
    ),
    (
        "12.001",
        DECODE,
        pulse_count_u32_rx,
        decode_pulse_count_u32(&[0, 0, 1, 2]),
        Ok(258)
    ),
    (
        "12.100",
        ENCODE,
        operating_seconds_tx,
        encode_operating_seconds(258),
        [0, 0, 1, 2]
    ),
    (
        "12.100",
        DECODE,
        operating_seconds_rx,
        decode_operating_seconds(&[0, 0, 1, 2]),
        Ok(258)
    ),
    (
        "12.101",
        ENCODE,
        operating_minutes_tx,
        encode_operating_minutes(258),
        [0, 0, 1, 2]
    ),
    (
        "12.101",
        DECODE,
        operating_minutes_rx,
        decode_operating_minutes(&[0, 0, 1, 2]),
        Ok(258)
    ),
    (
        "12.102",
        ENCODE,
        operating_hours_tx,
        encode_operating_hours(258),
        [0, 0, 1, 2]
    ),
    (
        "12.102",
        DECODE,
        operating_hours_rx,
        decode_operating_hours(&[0, 0, 1, 2]),
        Ok(258)
    ),
    (
        "13.001",
        ENCODE,
        pulse_count_i32_tx,
        encode_pulse_count_i32(-258),
        [255, 255, 254, 254]
    ),
    (
        "13.001",
        DECODE,
        pulse_count_i32_rx,
        decode_pulse_count_i32(&[255, 255, 254, 254]),
        Ok(-258)
    ),
    (
        "13.002",
        ENCODE,
        flow_rate_ticks_tx,
        encode_flow_rate_ticks(-258),
        [255, 255, 254, 254]
    ),
    (
        "13.002",
        DECODE,
        flow_rate_ticks_rx,
        decode_flow_rate_ticks(&[255, 255, 254, 254]),
        Ok(-258)
    ),
    (
        "13.100",
        ENCODE,
        operating_delta_seconds_tx,
        encode_operating_delta_seconds(-258),
        [255, 255, 254, 254]
    ),
    (
        "13.100",
        DECODE,
        operating_delta_seconds_rx,
        decode_operating_delta_seconds(&[255, 255, 254, 254]),
        Ok(-258)
    ),
    (
        "7.012",
        ENCODE,
        supply_ma_tx,
        encode_supply_ma(std::num::NonZeroU16::new(640)),
        [2, 128]
    ),
    (
        "7.012",
        DECODE,
        supply_ma_rx,
        decode_supply_ma(&[2, 128]),
        Ok(std::num::NonZeroU16::new(640))
    ),
    (
        "8.010",
        ENCODE,
        percent_hundredths_tx,
        encode_percent_hundredths(-258),
        Ok([254, 254])
    ),
    (
        "8.010",
        DECODE,
        percent_hundredths_rx,
        decode_percent_hundredths(&[254, 254]),
        Ok(-258)
    ),
    (
        "20.102",
        ENCODE,
        hvac_mode_tx,
        HvacMode::Economy.encode(),
        [3]
    ),
    (
        "20.102",
        DECODE,
        hvac_mode_rx,
        HvacMode::decode(&[4]),
        Ok(HvacMode::BuildingProtection)
    ),
    (
        "235.001",
        ENCODE,
        tariff_energy_tx,
        TariffEnergy {
            watt_hours: Some(-2),
            tariff: None
        }
        .encode(),
        [255, 255, 255, 254, 0, 1]
    ),
    (
        "235.001",
        DECODE,
        tariff_energy_rx,
        TariffEnergy::decode(&[255, 255, 255, 254, 4, 0]),
        Ok(TariffEnergy {
            watt_hours: Some(-2),
            tariff: Some(Tariff::numbered(4).unwrap())
        })
    ),
    (
        "7.002",
        ENCODE,
        period_ms_tx,
        TimeIntervalDpt::from_dpt("7.002").unwrap().encode_ms(258),
        Ok([1, 2])
    ),
    (
        "7.002",
        DECODE,
        period_ms_rx,
        TimeIntervalDpt::from_dpt("7.002")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(65_535)
    ),
    (
        "7.003",
        ENCODE,
        period_10ms_tx,
        TimeIntervalDpt::from_dpt("7.003").unwrap().encode_ms(2_580),
        Ok([1, 2])
    ),
    (
        "7.003",
        DECODE,
        period_10ms_rx,
        TimeIntervalDpt::from_dpt("7.003")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(655_350)
    ),
    (
        "7.004",
        ENCODE,
        period_100ms_tx,
        TimeIntervalDpt::from_dpt("7.004")
            .unwrap()
            .encode_ms(25_800),
        Ok([1, 2])
    ),
    (
        "7.004",
        DECODE,
        period_100ms_rx,
        TimeIntervalDpt::from_dpt("7.004")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(6_553_500)
    ),
    (
        "7.005",
        ENCODE,
        period_s_tx,
        TimeIntervalDpt::from_dpt("7.005")
            .unwrap()
            .encode_ms(258_000),
        Ok([1, 2])
    ),
    (
        "7.005",
        DECODE,
        period_s_rx,
        TimeIntervalDpt::from_dpt("7.005")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(65_535_000)
    ),
    (
        "7.006",
        ENCODE,
        period_min_tx,
        TimeIntervalDpt::from_dpt("7.006")
            .unwrap()
            .encode_ms(15_480_000),
        Ok([1, 2])
    ),
    (
        "7.006",
        DECODE,
        period_min_rx,
        TimeIntervalDpt::from_dpt("7.006")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(3_932_100_000)
    ),
    (
        "7.007",
        ENCODE,
        period_h_tx,
        TimeIntervalDpt::from_dpt("7.007")
            .unwrap()
            .encode_ms(928_800_000),
        Ok([1, 2])
    ),
    (
        "7.007",
        DECODE,
        period_h_rx,
        TimeIntervalDpt::from_dpt("7.007")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(235_926_000_000)
    ),
    (
        "8.002",
        ENCODE,
        delta_ms_tx,
        TimeIntervalDpt::from_dpt("8.002").unwrap().encode_ms(-258),
        Ok([0xfe, 0xfe])
    ),
    (
        "8.002",
        DECODE,
        delta_ms_rx,
        TimeIntervalDpt::from_dpt("8.002")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(-1)
    ),
    (
        "8.003",
        ENCODE,
        delta_10ms_tx,
        TimeIntervalDpt::from_dpt("8.003")
            .unwrap()
            .encode_ms(-2_580),
        Ok([0xfe, 0xfe])
    ),
    (
        "8.003",
        DECODE,
        delta_10ms_rx,
        TimeIntervalDpt::from_dpt("8.003")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(-10)
    ),
    (
        "8.004",
        ENCODE,
        delta_100ms_tx,
        TimeIntervalDpt::from_dpt("8.004")
            .unwrap()
            .encode_ms(-25_800),
        Ok([0xfe, 0xfe])
    ),
    (
        "8.004",
        DECODE,
        delta_100ms_rx,
        TimeIntervalDpt::from_dpt("8.004")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(-100)
    ),
    (
        "8.005",
        ENCODE,
        delta_s_tx,
        TimeIntervalDpt::from_dpt("8.005")
            .unwrap()
            .encode_ms(-258_000),
        Ok([0xfe, 0xfe])
    ),
    (
        "8.005",
        DECODE,
        delta_s_rx,
        TimeIntervalDpt::from_dpt("8.005")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(-1_000)
    ),
    (
        "8.006",
        ENCODE,
        delta_min_tx,
        TimeIntervalDpt::from_dpt("8.006")
            .unwrap()
            .encode_ms(-15_480_000),
        Ok([0xfe, 0xfe])
    ),
    (
        "8.006",
        DECODE,
        delta_min_rx,
        TimeIntervalDpt::from_dpt("8.006")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(-60_000)
    ),
    (
        "8.007",
        ENCODE,
        delta_h_tx,
        TimeIntervalDpt::from_dpt("8.007")
            .unwrap()
            .encode_ms(-928_800_000),
        Ok([0xfe, 0xfe])
    ),
    (
        "8.007",
        DECODE,
        delta_h_rx,
        TimeIntervalDpt::from_dpt("8.007")
            .unwrap()
            .decode_ms(&[255, 255]),
        Ok(-3_600_000)
    ),
    (
        "6.001",
        ENCODE,
        signed_percent_tx,
        encode_percent_i8(-128),
        [128]
    ),
    (
        "6.001",
        DECODE,
        signed_percent_rx,
        decode_percent_i8(&[255]),
        Ok(-1)
    ),
    (
        "6.010",
        ENCODE,
        signed_pulses_tx,
        encode_signed_pulse_count(-2),
        [254]
    ),
    (
        "6.010",
        DECODE,
        signed_pulses_rx,
        decode_signed_pulse_count(&[127]),
        Ok(127)
    ),
    (
        "6.020",
        ENCODE,
        status_mode_tx,
        StatusWithMode {
            set_flags: [true, false, true, false, true],
            mode: StatusMode::Mode1
        }
        .encode(),
        [0x52]
    ),
    (
        "6.020",
        DECODE,
        status_mode_rx,
        StatusWithMode::decode(&[0xfc]),
        Ok(StatusWithMode {
            set_flags: [false; 5],
            mode: StatusMode::Mode2
        })
    ),
    (
        "5.003",
        ENCODE,
        angle_tx,
        encode_angle_degrees(180.0),
        Ok([128])
    ),
    (
        "5.003",
        DECODE,
        angle_rx,
        decode_angle_degrees(&[255]),
        Ok(360.0)
    ),
    ("5.005", ENCODE, factor_tx, encode_decimal_factor(10), [10]),
    (
        "5.005",
        DECODE,
        factor_rx,
        decode_decimal_factor(&[255]),
        Ok(255)
    ),
    (
        "5.006",
        ENCODE,
        tariff_tx,
        Tariff::numbered(254).unwrap().encode(),
        [254]
    ),
    (
        "5.006",
        DECODE,
        tariff_rx,
        Tariff::decode(&[0]),
        Ok(Tariff::UNAVAILABLE)
    ),
    (
        "5.010",
        ENCODE,
        pulse_count_tx,
        encode_pulse_count(255),
        [255]
    ),
    (
        "5.010",
        DECODE,
        pulse_count_rx,
        decode_pulse_count(&[42]),
        Ok(42)
    ),
    (
        "2.001",
        ENCODE,
        priority_switch_tx,
        PriorityDpt::Switch.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.001",
        DECODE,
        priority_switch_rx,
        PriorityDpt::Switch.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.002",
        ENCODE,
        priority_boolean_tx,
        PriorityDpt::Boolean.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.002",
        DECODE,
        priority_boolean_rx,
        PriorityDpt::Boolean.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.003",
        ENCODE,
        priority_enable_tx,
        PriorityDpt::Enable.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.003",
        DECODE,
        priority_enable_rx,
        PriorityDpt::Enable.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.004",
        ENCODE,
        priority_ramp_tx,
        PriorityDpt::Ramp.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.004",
        DECODE,
        priority_ramp_rx,
        PriorityDpt::Ramp.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.005",
        ENCODE,
        priority_alarm_tx,
        PriorityDpt::Alarm.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.005",
        DECODE,
        priority_alarm_rx,
        PriorityDpt::Alarm.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.006",
        ENCODE,
        priority_binaryvalue_tx,
        PriorityDpt::BinaryValue.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.006",
        DECODE,
        priority_binaryvalue_rx,
        PriorityDpt::BinaryValue.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.007",
        ENCODE,
        priority_step_tx,
        PriorityDpt::Step.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.007",
        DECODE,
        priority_step_rx,
        PriorityDpt::Step.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.008",
        ENCODE,
        priority_updown_tx,
        PriorityDpt::UpDown.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.008",
        DECODE,
        priority_updown_rx,
        PriorityDpt::UpDown.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.009",
        ENCODE,
        priority_openclose_tx,
        PriorityDpt::OpenClose.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.009",
        DECODE,
        priority_openclose_rx,
        PriorityDpt::OpenClose.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.010",
        ENCODE,
        priority_start_tx,
        PriorityDpt::Start.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.010",
        DECODE,
        priority_start_rx,
        PriorityDpt::Start.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.011",
        ENCODE,
        priority_state_tx,
        PriorityDpt::State.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.011",
        DECODE,
        priority_state_rx,
        PriorityDpt::State.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "2.012",
        ENCODE,
        priority_invert_tx,
        PriorityDpt::Invert.encode(PriorityControl::Set(false)),
        [2]
    ),
    (
        "2.012",
        DECODE,
        priority_invert_rx,
        PriorityDpt::Invert.decode(&[1]),
        Ok(PriorityControl::Release)
    ),
    (
        "13.011",
        ENCODE,
        vah_32_tx,
        Energy32::ApparentVah.encode(16_909_060),
        [1, 2, 3, 4]
    ),
    (
        "13.011",
        DECODE,
        vah_32_rx,
        Energy32::ApparentVah.decode(&[255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "13.012",
        ENCODE,
        varh_32_tx,
        Energy32::ReactiveVarh.encode(16_909_060),
        [1, 2, 3, 4]
    ),
    (
        "13.012",
        DECODE,
        varh_32_rx,
        Energy32::ReactiveVarh.decode(&[255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "13.013",
        ENCODE,
        kwh_32_tx,
        Energy32::ActiveKwh.encode(16_909_060),
        [1, 2, 3, 4]
    ),
    (
        "13.013",
        DECODE,
        kwh_32_rx,
        Energy32::ActiveKwh.decode(&[255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "13.014",
        ENCODE,
        kvah_32_tx,
        Energy32::ApparentKvah.encode(16_909_060),
        [1, 2, 3, 4]
    ),
    (
        "13.014",
        DECODE,
        kvah_32_rx,
        Energy32::ApparentKvah.decode(&[255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "13.015",
        ENCODE,
        kvarh_32_tx,
        Energy32::ReactiveKvarh.encode(16_909_060),
        [1, 2, 3, 4]
    ),
    (
        "13.015",
        DECODE,
        kvarh_32_rx,
        Energy32::ReactiveKvarh.decode(&[255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "13.016",
        ENCODE,
        mwh_32_tx,
        Energy32::ActiveMwh.encode(16_909_060),
        [1, 2, 3, 4]
    ),
    (
        "13.016",
        DECODE,
        mwh_32_rx,
        Energy32::ActiveMwh.decode(&[255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "29.010",
        ENCODE,
        wh_64_tx,
        Energy64::ActiveWh.encode(9_007_199_254_740_993),
        [0, 32, 0, 0, 0, 0, 0, 1]
    ),
    (
        "29.010",
        DECODE,
        wh_64_rx,
        Energy64::ActiveWh.decode(&[255, 255, 255, 255, 255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "29.011",
        ENCODE,
        vah_64_tx,
        Energy64::ApparentVah.encode(9_007_199_254_740_993),
        [0, 32, 0, 0, 0, 0, 0, 1]
    ),
    (
        "29.011",
        DECODE,
        vah_64_rx,
        Energy64::ApparentVah.decode(&[255, 255, 255, 255, 255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "29.012",
        ENCODE,
        varh_64_tx,
        Energy64::ReactiveVarh.encode(9_007_199_254_740_993),
        [0, 32, 0, 0, 0, 0, 0, 1]
    ),
    (
        "29.012",
        DECODE,
        varh_64_rx,
        Energy64::ReactiveVarh.decode(&[255, 255, 255, 255, 255, 255, 255, 254]),
        Ok(-2)
    ),
    (
        "3.007",
        ENCODE,
        dimming_tx,
        encode_dimming(Dimming::Increase(StepSize::from_denominator(4).unwrap())),
        [0x0b]
    ),
    (
        "3.007",
        DECODE,
        dimming_rx,
        decode_dimming(&[0x08]),
        Ok(Dimming::Stop)
    ),
    (
        "3.008",
        ENCODE,
        blinds_tx,
        encode_blind_control(BlindControl::Up(StepSize::from_denominator(2).unwrap())),
        [0x02]
    ),
    (
        "3.008",
        DECODE,
        blinds_rx,
        decode_blind_control(&[0x0f]),
        Ok(BlindControl::Down(StepSize::from_denominator(64).unwrap()))
    ),
    (
        "17.001",
        ENCODE,
        scene_number_tx,
        encode_scene_number(SceneNumber::new(1).unwrap()),
        [0]
    ),
    (
        "17.001",
        DECODE,
        scene_number_rx,
        decode_scene_number(&[63]),
        SceneNumber::new(64)
    ),
    (
        "18.001",
        ENCODE,
        scene_control_tx,
        encode_scene_control(SceneControl::Learn(SceneNumber::new(64).unwrap())),
        [0xbf]
    ),
    (
        "18.001",
        DECODE,
        scene_control_rx,
        decode_scene_control(&[0]),
        Ok(SceneControl::Activate(SceneNumber::new(1).unwrap()))
    ),
    ("1.001", ENCODE, switch_tx, encode_switch(true), [1]),
    ("1.001", DECODE, switch_rx, decode_switch(&[1]), Ok(true)),
    (
        "5.001",
        ENCODE,
        scaling_tx,
        encode_scaling_percent(100.0),
        Ok([255])
    ),
    (
        "5.001",
        DECODE,
        scaling_rx,
        decode_scaling_percent(&[255]),
        Ok(100.0)
    ),
    (
        "5.004",
        ENCODE,
        whole_percent_tx,
        encode_percent_u8(255),
        [255]
    ),
    (
        "5.004",
        DECODE,
        whole_percent_rx,
        decode_percent_u8(&[255]),
        Ok(255)
    ),
    (
        "9.001",
        ENCODE,
        temperature_tx,
        encode_temperature_celsius(20.0),
        Ok([7, 208])
    ),
    (
        "9.001",
        DECODE,
        temperature_rx,
        decode_temperature_celsius(&[7, 208]),
        Ok(20.0)
    ),
    ("13.010", ENCODE, energy_tx, encode_energy_wh(-1), [255; 4]),
    (
        "13.010",
        DECODE,
        energy_rx,
        decode_energy_wh(&[255; 4]),
        Ok(-1)
    ),
    (
        "4.001",
        ENCODE,
        ascii_char_tx,
        Charset::Ascii.encode_char('A'),
        Ok([65])
    ),
    (
        "4.001",
        DECODE,
        ascii_char_rx,
        Charset::Ascii.decode_char(&[65]),
        Ok('A')
    ),
    (
        "4.002",
        ENCODE,
        latin_char_tx,
        Charset::Latin1.encode_char('ß'),
        Ok([223])
    ),
    (
        "4.002",
        DECODE,
        latin_char_rx,
        Charset::Latin1.decode_char(&[223]),
        Ok('ß')
    ),
    (
        "10.001",
        ENCODE,
        time_of_day_tx,
        TimeOfDay {
            weekday: 7,
            time: Time {
                hour: 23,
                minute: 59,
                second: 58
            }
        }
        .encode(),
        Ok([247, 59, 58])
    ),
    (
        "10.001",
        DECODE,
        time_of_day_rx,
        TimeOfDay::decode(&[247, 59, 58]),
        Ok(TimeOfDay {
            weekday: 7,
            time: Time {
                hour: 23,
                minute: 59,
                second: 58
            }
        })
    ),
    (
        "11.001",
        ENCODE,
        calendar_date_tx,
        Date {
            year: 1990,
            month: 12,
            day: 31
        }
        .encode(),
        Ok([31, 12, 90])
    ),
    (
        "11.001",
        DECODE,
        calendar_date_rx,
        Date::decode(&[29, 2, 0]),
        Ok(Date {
            year: 2000,
            month: 2,
            day: 29
        })
    ),
    (
        "16.000",
        ENCODE,
        fixed_ascii_tx,
        Charset::Ascii.encode_fixed("KNX is OK"),
        Ok(*b"KNX is OK\0\0\0\0\0")
    ),
    (
        "16.000",
        DECODE,
        fixed_ascii_rx,
        Charset::Ascii.decode_fixed(b"KNX is OK\0\0\0\0\0"),
        Ok(b"KNX is OK".as_slice())
    ),
    (
        "16.001",
        ENCODE,
        fixed_latin_tx,
        Charset::Latin1.encode_fixed("Ä"),
        Ok([196, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    ),
    (
        "16.001",
        DECODE,
        fixed_latin_rx,
        Charset::Latin1.decode_fixed(&[196, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Ok([196].as_slice())
    ),
    (
        "19.001",
        ENCODE,
        calendar_fields_tx,
        encode_date_time(DateTime::Value(CalendarFields {
            month_day: Some((2, 29)),
            ..Default::default()
        })),
        Ok([0, 2, 29, 0, 0, 0, 0x36, 0])
    ),
    (
        "19.001",
        DECODE,
        calendar_fields_rx,
        decode_date_time(&[0, 2, 29, 0, 0, 0, 0x36, 0]),
        Ok(DateTime::Value(CalendarFields {
            month_day: Some((2, 29)),
            ..Default::default()
        }))
    ),
    (
        "24.001",
        ENCODE,
        variable_latin_tx,
        encode_latin1("°C", &mut [0; 3]),
        Ok([176, 67, 0].as_slice())
    ),
    (
        "24.001",
        DECODE,
        variable_latin_rx,
        decode_latin1(&[176, 67, 0]),
        Ok([176, 67].as_slice())
    ),
    (
        "28.001",
        ENCODE,
        variable_utf8_tx,
        encode_utf8("°C", &mut [0; 4]),
        Ok([194, 176, 67, 0].as_slice())
    ),
    (
        "28.001",
        DECODE,
        variable_utf8_rx,
        decode_utf8(&[194, 176, 67, 0]),
        Ok("°C")
    ),
    (
        "14.056",
        ENCODE,
        power_tx,
        encode_power_watts(-24000.0),
        Ok([198, 187, 128, 0])
    ),
    (
        "14.056",
        DECODE,
        power_rx,
        decode_power_watts(&[198, 187, 128, 0]),
        Ok(-24000.0)
    ),
);

#[test]
fn edition_and_support_are_traceable() {
    let mut ids = HashSet::new();
    let mut claims = HashSet::new();
    let mut layout_claims = HashSet::new();
    let mut counts = [0; 4];
    let mut lines = CATALOG.lines();
    assert_eq!(
        lines.next(),
        Some(
            "edition,kind,id,name,section,owner,encode,decode,encode_evidence,decode_evidence,parameter_binding,parameter_validation"
        )
    );
    for line in lines {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 12);
        assert_eq!(fields[0], "02.02.01");
        let kind = match fields[1] {
            "dpt" => 0,
            "parameter" => 1,
            "unassigned-parameter" => 2,
            "legacy" => 3,
            other => panic!("unrecognized inventory kind {other}"),
        };
        counts[kind] += 1;
        assert_eq!(fields[2].is_empty(), kind >= 2);
        let key = if kind >= 2 { fields[3] } else { fields[2] };
        assert!(ids.insert((kind, key)), "duplicate {key}");
        assert!(fields[3..6].iter().all(|s| !s.is_empty()));
        for direction in [ENCODE, DECODE] {
            let evidence = fields[direction + 2];
            match fields[direction] {
                "tested" => {
                    assert_eq!(kind, 0);
                    claims.insert((key, direction, evidence));
                }
                "not-applicable" => {
                    assert!(matches!(kind, 1 | 2));
                    assert!(evidence.is_empty());
                    assert!(matches!(fields[11], "value" | "layout" | "unspecified"));
                }
                "layout-tested" => {
                    assert_eq!(kind, 0);
                    layout_claims.insert((key, direction, evidence));
                }
                "unsupported" => assert!(evidence.is_empty()),
                other => panic!("unrecognized support status {other}"),
            }
        }
    }
    assert_eq!(
        layout_claims,
        register_layout::EVIDENCE
            .iter()
            .chain(MBUS_LAYOUT)
            .copied()
            .collect()
    );
    assert_eq!(counts, [454, 32, 4, 11]);
    assert_eq!(
        claims,
        EVIDENCE
            .iter()
            .chain(structured_vectors::EVIDENCE)
            .chain(short_float::EVIDENCE)
            .chain(long_float::EVIDENCE)
            .copied()
            .collect()
    );
}

use super::{DECODE, ENCODE};
use knx::{SceneNumber, structured::*, text_time::DateTime};

macro_rules! cases {
    ($(($id:literal, $tx:ident, $rx:ident, $value:ident=$sample:expr, $wire:ident=$bytes:expr, $encode:expr, $decode:expr)),+ $(,)?) => {
        $(
            #[test] fn $tx(){let $value=$sample;assert_eq!($encode,$bytes);}
            #[test] fn $rx(){let $wire=$bytes;assert_eq!($decode,$sample);}
        )+
        pub(super) const EVIDENCE:&[(&str,usize,&str)]=&[
            $(($id,ENCODE,concat!("tests/inventory.rs::structured_vectors::", stringify!($tx))),
              ($id,DECODE,concat!("tests/inventory.rs::structured_vectors::", stringify!($rx))),)+
        ];
    }
}

cases!(
    (
        "200.100",
        dpt_200_100_tx,
        dpt_200_100_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("200.100")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("200.100")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "200.101",
        dpt_200_101_tx,
        dpt_200_101_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("200.101")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("200.101")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "201.100",
        dpt_201_100_tx,
        dpt_201_100_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("201.100")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("201.100")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "201.102",
        dpt_201_102_tx,
        dpt_201_102_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("201.102")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("201.102")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "201.104",
        dpt_201_104_tx,
        dpt_201_104_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("201.104")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("201.104")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "201.105",
        dpt_201_105_tx,
        dpt_201_105_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("201.105")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("201.105")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "201.107",
        dpt_201_107_tx,
        dpt_201_107_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("201.107")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("201.107")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "201.108",
        dpt_201_108_tx,
        dpt_201_108_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("201.108")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("201.108")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "201.109",
        dpt_201_109_tx,
        dpt_201_109_rx,
        value = LteValue::Overridden {
            ticks: 1,
            fault: false
        },
        wire = [1, 4],
        {
            let mut buf = [0; 2];
            LteDpt::from_dpt("201.109")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("201.109")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.100",
        dpt_203_100_tx,
        dpt_203_100_rx,
        value = LteValue::Overridden {
            ticks: 258,
            fault: false
        },
        wire = [1, 2, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("203.100")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("203.100")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.101",
        dpt_203_101_tx,
        dpt_203_101_rx,
        value = LteValue::Overridden {
            ticks: 258,
            fault: false
        },
        wire = [1, 2, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("203.101")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("203.101")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.102",
        dpt_203_102_tx,
        dpt_203_102_rx,
        value = LteValue::Overridden {
            ticks: 258,
            fault: false
        },
        wire = [1, 2, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("203.102")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("203.102")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.104",
        dpt_203_104_tx,
        dpt_203_104_rx,
        value = LteValue::Overridden {
            ticks: 258,
            fault: false
        },
        wire = [1, 2, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("203.104")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("203.104")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.100",
        dpt_205_100_tx,
        dpt_205_100_rx,
        value = LteValue::Overridden {
            ticks: -42,
            fault: false
        },
        wire = [255, 214, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("205.100")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("205.100")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.101",
        dpt_205_101_tx,
        dpt_205_101_rx,
        value = LteValue::Overridden {
            ticks: -42,
            fault: false
        },
        wire = [255, 214, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("205.101")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("205.101")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.102",
        dpt_205_102_tx,
        dpt_205_102_rx,
        value = LteValue::Overridden {
            ticks: -42,
            fault: false
        },
        wire = [255, 214, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("205.102")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("205.102")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.103",
        dpt_205_103_tx,
        dpt_205_103_rx,
        value = LteValue::Overridden {
            ticks: -42,
            fault: false
        },
        wire = [255, 214, 4],
        {
            let mut buf = [0; 3];
            LteDpt::from_dpt("205.103")
                .unwrap()
                .encode_report(value, &mut buf)
                .unwrap();
            buf
        },
        LteDpt::from_dpt("205.103")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "20.001",
        dpt_20_001_tx,
        dpt_20_001_rx,
        value = SystemClockMode::Master,
        wire = [2],
        value.encode(),
        SystemClockMode::decode(&wire).unwrap()
    ),
    (
        "20.002",
        dpt_20_002_tx,
        dpt_20_002_rx,
        value = BuildingMode::Protection,
        wire = [2],
        value.encode(),
        BuildingMode::decode(&wire).unwrap()
    ),
    (
        "20.003",
        dpt_20_003_tx,
        dpt_20_003_rx,
        value = OccupancyMode::Unoccupied,
        wire = [2],
        value.encode(),
        OccupancyMode::decode(&wire).unwrap()
    ),
    (
        "20.004",
        dpt_20_004_tx,
        dpt_20_004_rx,
        value = ParameterPriority::Void,
        wire = [3],
        value.encode(),
        ParameterPriority::decode(&wire).unwrap()
    ),
    (
        "20.006",
        dpt_20_006_tx,
        dpt_20_006_rx,
        value = ApplicationArea::ShuttersBlinds,
        wire = [50],
        value.encode(),
        ApplicationArea::decode(&wire).unwrap()
    ),
    (
        "20.007",
        dpt_20_007_tx,
        dpt_20_007_rx,
        value = AlarmClass::Extended,
        wire = [3],
        value.encode(),
        AlarmClass::decode(&wire).unwrap()
    ),
    (
        "20.008",
        dpt_20_008_tx,
        dpt_20_008_rx,
        value = PowerSupplyMode::Automatic,
        wire = [2],
        value.encode(),
        PowerSupplyMode::decode(&wire).unwrap()
    ),
    (
        "20.011",
        dpt_20_011_tx,
        dpt_20_011_rx,
        value = SystemFault::GroupObjectTypeTooLong,
        wire = [18],
        value.encode(),
        SystemFault::decode(&wire).unwrap()
    ),
    (
        "20.012",
        dpt_20_012_tx,
        dpt_20_012_rx,
        value = HvacFault::Other,
        wire = [4],
        value.encode(),
        HvacFault::decode(&wire).unwrap()
    ),
    (
        "20.014",
        dpt_20_014_tx,
        dpt_20_014_rx,
        value = WindForce::Hurricane,
        wire = [12],
        value.encode(),
        WindForce::decode(&wire).unwrap()
    ),
    (
        "20.017",
        dpt_20_017_tx,
        dpt_20_017_rx,
        value = SensorSelection::Temperature,
        wire = [4],
        value.encode(),
        SensorSelection::decode(&wire).unwrap()
    ),
    (
        "20.020",
        dpt_20_020_tx,
        dpt_20_020_rx,
        value = ActuatorConnection::Controller,
        wire = [2],
        value.encode(),
        ActuatorConnection::decode(&wire).unwrap()
    ),
    (
        "20.021",
        dpt_20_021_tx,
        dpt_20_021_rx,
        value = CloudCover::Obstructed,
        wire = [9],
        value.encode(),
        CloudCover::decode(&wire).unwrap()
    ),
    (
        "20.022",
        dpt_20_022_tx,
        dpt_20_022_rx,
        value = PowerReturnMode::SendChanged,
        wire = [2],
        value.encode(),
        PowerReturnMode::decode(&wire).unwrap()
    ),
    (
        "23.001",
        dpt_23_001_tx,
        dpt_23_001_rx,
        value = SwitchAction::OnThenOff,
        wire = [3],
        value.encode(),
        SwitchAction::decode(&wire).unwrap()
    ),
    (
        "23.002",
        dpt_23_002_tx,
        dpt_23_002_rx,
        value = AlarmReaction::Down,
        wire = [2],
        value.encode(),
        AlarmReaction::decode(&wire).unwrap()
    ),
    (
        "23.003",
        dpt_23_003_tx,
        dpt_23_003_rx,
        value = TravelAction::DownThenUp,
        wire = [3],
        value.encode(),
        TravelAction::decode(&wire).unwrap()
    ),
    (
        "20.005",
        dpt_20_005_tx,
        dpt_20_005_rx,
        value = LightApplicationMode::Manufacturer(37),
        wire = [37],
        value.encode().unwrap(),
        LightApplicationMode::decode(&wire).unwrap()
    ),
    (
        "20.013",
        dpt_20_013_tx,
        dpt_20_013_rx,
        value = 75,
        wire = [11],
        encode_time_delay_seconds(value).unwrap(),
        decode_time_delay_seconds(&wire).unwrap()
    ),
    (
        "21.001",
        dpt_21_001_tx,
        dpt_21_001_rx,
        value = GeneralStatus {
            out_of_service: false,
            fault: true,
            overridden: true,
            in_alarm: true,
            alarm_unacknowledged: false
        },
        wire = [14],
        value.encode().unwrap(),
        GeneralStatus::decode(&wire).unwrap()
    ),
    (
        "21.002",
        dpt_21_002_tx,
        dpt_21_002_rx,
        value = DeviceControl {
            user_stopped: true,
            own_address_received: false,
            verify_mode: true
        },
        wire = [5],
        value.encode(),
        DeviceControl::decode(&wire).unwrap()
    ),
    (
        "26.001",
        dpt_26_001_tx,
        dpt_26_001_rx,
        value = SceneInfo {
            scene: SceneNumber::new(64).unwrap(),
            active: false
        },
        wire = [127],
        value.encode(),
        SceneInfo::decode(&wire).unwrap()
    ),
    (
        "238.001",
        dpt_238_001_tx,
        dpt_238_001_rx,
        value = SceneConfig {
            info: SceneInfo {
                scene: SceneNumber::new(64).unwrap(),
                active: true
            },
            storage_enabled: false
        },
        wire = [191],
        value.encode(),
        SceneConfig::decode(&wire).unwrap()
    ),
    (
        "27.001",
        dpt_27_001_tx,
        dpt_27_001_rx,
        value = std::array::from_fn(|i| match i {
            0 => Some(true),
            15 => Some(false),
            _ => None,
        }),
        wire = [128, 1, 0, 1],
        encode_output_states(value),
        decode_output_states(&wire).unwrap()
    ),
    (
        "221.001",
        dpt_221_001_tx,
        dpt_221_001_rx,
        value = SerialNumber {
            manufacturer: 0x1234,
            number: 0x56789abc
        },
        wire = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc],
        value.encode(),
        SerialNumber::decode(&wire).unwrap()
    ),
    (
        "15.000",
        dpt_15_000_tx,
        dpt_15_000_rx,
        value = AccessData {
            digits: [1, 2, 3, 4, 5, 6],
            read_error: false,
            accepted: true,
            right_to_left: false,
            encrypted: false,
            index: 13
        },
        wire = [0x12, 0x34, 0x56, 0x4d],
        value.encode().unwrap(),
        AccessData::decode(&wire).unwrap()
    ),
    (
        "219.001",
        dpt_219_001_tx,
        dpt_219_001_rx,
        value = AlarmInfo {
            log_number: 42,
            priority: ParameterPriority::High,
            area: ApplicationArea::System,
            error_class: 2,
            can_acknowledge: true,
            has_timestamp: false,
            has_alarm_text: true,
            has_error_code: false,
            in_alarm: true,
            unacknowledged: true,
            locked: false
        },
        wire = [42, 0, 1, 2, 5, 3],
        value.encode().unwrap(),
        AlarmInfo::decode(&wire).unwrap()
    ),
    (
        "217.001",
        dpt_217_001_tx,
        dpt_217_001_rx,
        value = Some(Version {
            magic: 1,
            version: 3,
            revision: 5
        }),
        wire = [8, 197],
        Version::encode(value).unwrap(),
        Version::decode(&wire).unwrap()
    ),
    (
        "225.003",
        dpt_225_003_tx,
        dpt_225_003_rx,
        value = Some(NextTariff {
            delay_minutes: std::num::NonZeroU16::new(257),
            tariff: 23
        }),
        wire = [1, 1, 23],
        NextTariff::encode(value).unwrap(),
        NextTariff::decode(&wire).unwrap()
    ),
    (
        "236.001",
        dpt_236_001_tx,
        dpt_236_001_rx,
        value = PrioritisedMode::Activate {
            priority: 3,
            mode: 12
        },
        wire = [60],
        value.encode().unwrap(),
        PrioritisedMode::decode(&wire).unwrap()
    ),
    (
        "239.001",
        dpt_239_001_tx,
        dpt_239_001_rx,
        value = FlaggedScaling {
            scaling: 127,
            channel_active: true
        },
        wire = [127, 1],
        value.encode(),
        FlaggedScaling::decode(&wire).unwrap()
    ),
    (
        "255.001",
        dpt_255_001_tx,
        dpt_255_001_rx,
        value = Location {
            longitude_degrees: 1.0,
            latitude_degrees: -2.5
        },
        wire = [63, 128, 0, 0, 192, 32, 0, 0],
        value.encode().unwrap(),
        Location::decode(&wire).unwrap()
    ),
    (
        "225.001",
        dpt_225_001_tx,
        dpt_225_001_rx,
        value = ScalingTiming {
            milliseconds: 4000,
            scaling_code: std::num::NonZeroU8::new(255).unwrap()
        },
        wire = [0, 40, 255],
        ScalingTimingDpt::Speed.encode(value).unwrap(),
        ScalingTimingDpt::Speed.decode(&wire).unwrap()
    ),
    (
        "225.002",
        dpt_225_002_tx,
        dpt_225_002_rx,
        value = ScalingTiming {
            milliseconds: 15,
            scaling_code: std::num::NonZeroU8::new(255).unwrap()
        },
        wire = [0, 15, 255],
        ScalingTimingDpt::StepTime.encode(value).unwrap(),
        ScalingTimingDpt::StepTime.decode(&wire).unwrap()
    ),
    (
        "234.001",
        dpt_234_001_tx,
        dpt_234_001_rx,
        value = *b"en",
        wire = *b"en",
        Alpha2::Language.normalize(&value).unwrap(),
        Alpha2::Language.normalize(&wire).unwrap()
    ),
    (
        "234.002",
        dpt_234_002_tx,
        dpt_234_002_rx,
        value = *b"GB",
        wire = *b"GB",
        Alpha2::Region.normalize(&value).unwrap(),
        Alpha2::Region.normalize(&wire).unwrap()
    ),
    (
        "231.001",
        dpt_231_001_tx,
        dpt_231_001_rx,
        value = *b"enGB",
        wire = *b"enGB",
        normalize_locale(&value).unwrap(),
        normalize_locale(&wire).unwrap()
    ),
    (
        "202.001",
        dpt_202_001_tx,
        dpt_202_001_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [42, 4],
        {
            let mut out = [0; 2];
            LteDpt::from_dpt("202.001")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("202.001")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "202.002",
        dpt_202_002_tx,
        dpt_202_002_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [42, 4],
        {
            let mut out = [0; 2];
            LteDpt::from_dpt("202.002")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("202.002")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.002",
        dpt_203_002_tx,
        dpt_203_002_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.002")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.002")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.003",
        dpt_203_003_tx,
        dpt_203_003_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.003")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.003")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.004",
        dpt_203_004_tx,
        dpt_203_004_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.004")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.004")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.005",
        dpt_203_005_tx,
        dpt_203_005_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.005")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.005")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.006",
        dpt_203_006_tx,
        dpt_203_006_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.006")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.006")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.007",
        dpt_203_007_tx,
        dpt_203_007_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.007")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.007")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.011",
        dpt_203_011_tx,
        dpt_203_011_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.011")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.011")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.012",
        dpt_203_012_tx,
        dpt_203_012_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.012")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.012")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.013",
        dpt_203_013_tx,
        dpt_203_013_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.013")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.013")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.014",
        dpt_203_014_tx,
        dpt_203_014_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.014")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.014")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.015",
        dpt_203_015_tx,
        dpt_203_015_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.015")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.015")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "203.017",
        dpt_203_017_tx,
        dpt_203_017_rx,
        value = LteValue::Overridden {
            ticks: 42,
            fault: false
        },
        wire = [0, 42, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("203.017")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("203.017")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "204.001",
        dpt_204_001_tx,
        dpt_204_001_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [254, 4],
        {
            let mut out = [0; 2];
            LteDpt::from_dpt("204.001")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("204.001")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.002",
        dpt_205_002_tx,
        dpt_205_002_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 254, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("205.002")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("205.002")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.003",
        dpt_205_003_tx,
        dpt_205_003_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 254, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("205.003")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("205.003")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.004",
        dpt_205_004_tx,
        dpt_205_004_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 254, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("205.004")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("205.004")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.005",
        dpt_205_005_tx,
        dpt_205_005_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 254, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("205.005")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("205.005")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.006",
        dpt_205_006_tx,
        dpt_205_006_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 254, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("205.006")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("205.006")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.007",
        dpt_205_007_tx,
        dpt_205_007_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 254, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("205.007")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("205.007")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "205.017",
        dpt_205_017_tx,
        dpt_205_017_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 254, 4],
        {
            let mut out = [0; 3];
            LteDpt::from_dpt("205.017")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("205.017")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "218.001",
        dpt_218_001_tx,
        dpt_218_001_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 255, 255, 254, 4],
        {
            let mut out = [0; 5];
            LteDpt::from_dpt("218.001")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("218.001")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "218.002",
        dpt_218_002_tx,
        dpt_218_002_rx,
        value = LteValue::Overridden {
            ticks: -2,
            fault: false
        },
        wire = [255, 255, 255, 254, 4],
        {
            let mut out = [0; 5];
            LteDpt::from_dpt("218.002")
                .unwrap()
                .encode_report(value, &mut out)
                .unwrap();
            out
        },
        LteDpt::from_dpt("218.002")
            .unwrap()
            .decode_report(&wire)
            .unwrap()
    ),
    (
        "229.001",
        dpt_229_001_tx,
        dpt_229_001_rx,
        value = MeteringValue {
            value: LteValue::Measured(-2),
            scale: MeterScale::from_code(0x80).unwrap()
        },
        wire = [255, 255, 255, 254, 128, 0],
        value.encode_report().unwrap(),
        MeteringValue::decode_report(&wire).unwrap()
    ),
    (
        "256.001",
        dpt_256_001_tx,
        dpt_256_001_rx,
        value = DateTimePeriod {
            start: DateTime::Fault,
            stop: DateTime::Fault
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 128, 0],
        value.encode().unwrap(),
        DateTimePeriod::decode(&wire).unwrap()
    ),
    (
        "265.001",
        dpt_265_001_tx,
        dpt_265_001_rx,
        value = Dated {
            at: DateTime::Fault,
            value: "on"
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 1],
        DatedBinary::from_dpt("265.001")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedBinary::from_dpt("265.001")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "265.005",
        dpt_265_005_tx,
        dpt_265_005_rx,
        value = Dated {
            at: DateTime::Fault,
            value: "alarm"
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 1],
        DatedBinary::from_dpt("265.005")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedBinary::from_dpt("265.005")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "265.009",
        dpt_265_009_tx,
        dpt_265_009_rx,
        value = Dated {
            at: DateTime::Fault,
            value: "close"
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 1],
        DatedBinary::from_dpt("265.009")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedBinary::from_dpt("265.009")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "265.011",
        dpt_265_011_tx,
        dpt_265_011_rx,
        value = Dated {
            at: DateTime::Fault,
            value: "active"
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 1],
        DatedBinary::from_dpt("265.011")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedBinary::from_dpt("265.011")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "265.012",
        dpt_265_012_tx,
        dpt_265_012_rx,
        value = Dated {
            at: DateTime::Fault,
            value: "inverted"
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 1],
        DatedBinary::from_dpt("265.012")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedBinary::from_dpt("265.012")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "266.027",
        dpt_266_027_tx,
        dpt_266_027_rx,
        value = Dated {
            at: DateTime::Fault,
            value: -2.5
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 192, 32, 0, 0],
        DatedMeasurement::from_dpt("266.027")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedMeasurement::from_dpt("266.027")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "266.056",
        dpt_266_056_tx,
        dpt_266_056_rx,
        value = Dated {
            at: DateTime::Fault,
            value: -2.5
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 192, 32, 0, 0],
        DatedMeasurement::from_dpt("266.056")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedMeasurement::from_dpt("266.056")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "266.080",
        dpt_266_080_tx,
        dpt_266_080_rx,
        value = Dated {
            at: DateTime::Fault,
            value: -2.5
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 192, 32, 0, 0],
        DatedMeasurement::from_dpt("266.080")
            .unwrap()
            .encode(value)
            .unwrap(),
        DatedMeasurement::from_dpt("266.080")
            .unwrap()
            .decode(&wire)
            .unwrap()
    ),
    (
        "267.001",
        dpt_267_001_tx,
        dpt_267_001_rx,
        value = Dated {
            at: DateTime::Fault,
            value: "é"
        },
        wire = [0, 0, 0, 0, 0, 0, 128, 0, 195, 169, 0],
        {
            let mut out = [0; 11];
            encode_dated_text(value, &mut out).unwrap();
            out
        },
        decode_dated_text(&wire).unwrap()
    ),
);

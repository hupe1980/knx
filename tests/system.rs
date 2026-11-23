use knx::{CodecError, hvac_flags::*, system::*};

#[test]
fn valid_enums_and_counter_pairs_round_trip_all_octets() {
    for v in u8::MIN..=u8::MAX {
        let data = [v];
        macro_rules! verify {
            ($kind:ty, $allowed:expr) => {{
                let result = <$kind>::decode(&data);
                assert_eq!(result.is_ok(), $allowed);
                if let Ok(value) = result {
                    assert_eq!(value.encode(), data);
                }
            }};
        }
        verify!(CommunicationMode, matches!(v, 0..=2 | 6 | 255));
        verify!(AdditionalInfoType, (1..=7).contains(&v));
        verify!(RadioMode, v <= 2);
        verify!(RadioFilter, v <= 3);
        verify!(MediumType, matches!(v, 0..=2 | 5));
        verify!(ButtonFunction, (1..=55).contains(&v) && v != 15);
        verify!(RadioModeSupport, v <= 7 && v & 1 == 1);
        verify!(RadioFilterSupport, v <= 7);
        verify!(SecurityReport, v <= 1);
        let reps = Repetitions::decode(&data);
        assert_eq!(reps.is_ok(), v >> 4 <= 3 && v & 15 <= 3);
        if let Ok(reps) = reps {
            assert_eq!(reps.encode(), Ok(data))
        }
        assert_eq!(decode_channels_8(&encode_channels_8(v)), Ok(v));
    }
    assert_eq!(
        Repetitions {
            busy: 1,
            negative_ack: 4
        }
        .encode(),
        Err(CodecError::OutsideRange)
    );
    assert_eq!(
        Repetitions {
            busy: 4,
            negative_ack: 0
        }
        .encode(),
        Err(CodecError::OutsideRange)
    );
}

#[test]
fn full_width_and_reserved_bits_are_enforced() {
    for value in u16::MIN..=u16::MAX {
        let data = value.to_be_bytes();
        let media = SupportedMedia::decode(&data);
        assert_eq!(media.is_ok(), value & !0x36 == 0);
        if let Ok(media) = media {
            assert_eq!(media.encode(), data);
        }
        assert_eq!(decode_channels_16(&encode_channels_16(value)), Ok(value));
        let status = RoomClimateStatus::decode(&data);
        assert_eq!(status.is_ok(), value < 0x8000);
        if let Ok(status) = status {
            assert_eq!(status.encode(), data)
        }
        let water = HotWaterStatus::decode(&data);
        assert_eq!(water.is_ok(), value <= 255);
        if let Ok(water) = water {
            assert_eq!(water.encode(), data)
        }
    }
    assert_eq!(RoomClimateStatus::default().encode(), [1, 0]);
    assert_eq!(encode_channels_24(0xff_ffff), Ok([255; 3]));
    assert_eq!(
        encode_channels_24(0x0100_0000),
        Err(CodecError::OutsideRange)
    );
    assert_eq!(
        MBusAddress::decode(&[0; 7]),
        Err(CodecError::PayloadLength {
            expected: 8,
            actual: 7
        })
    );
    let raw = [255; 8];
    assert_eq!(MBusAddress::decode(&raw).unwrap().encode(), raw);
}

#[test]
fn hvac_octet_sets_preserve_all_assigned_flags() {
    for value in u8::MIN..=u8::MAX {
        let data = [value];
        macro_rules! check {
            ($kind:ty,$allowed:expr) => {{
                let flags = <$kind>::decode(&data);
                assert_eq!(flags.is_ok(), value & !$allowed == 0);
                if let Ok(flags) = flags {
                    assert_eq!(flags.encode(), data)
                }
            }};
        }
        check!(HeatForcing, 255u8);
        check!(CoolForcing, 1u8);
        check!(RoomHeatingStatus, 255u8);
        check!(SolarHotWaterStatus, 7u8);
        check!(FuelTypes, 7u8);
        check!(RoomCoolingStatus, 1u8);
        check!(VentilationStatus, 15u8);
    }
}

#[test]
fn manufacturer_flags_match_independent_meter_headers_in_knx_byte_order() {
    // M-Bus Usergroup §6.3 PAD and Kamstrup 55121890_F1_GB §4.3 KAM.
    for (wire, flag) in [
        ([0x40, 0x24, 0x12, 0x34, 0x56, 0x78, 1, 7], "PAD"),
        ([0x2c, 0x2d, 0x71, 0x00, 0x37, 0x88, 0x34, 4], "KAM"),
    ] {
        let address = MBusAddress::decode(&wire).unwrap();
        assert_eq!(address.manufacturer_flag().unwrap().as_str(), flag);
        let parsed: ManufacturerFlag = flag.to_ascii_lowercase().parse().unwrap();
        assert_eq!(parsed.to_string(), flag);
        assert_eq!(u16::from(parsed), address.manufacturer);
        assert_eq!(address.encode(), wire);
    }
    for invalid in ["", "AB", "ABCD", "A1C", "ÀZ", "A C"] {
        assert!(invalid.parse::<ManufacturerFlag>().is_err());
    }
}

#[test]
fn unrecognized_manufacturer_words_remain_lossless_without_a_flag() {
    let mut recognized = 0;
    for code in 0..=u16::MAX {
        let [hi, lo] = code.to_be_bytes();
        let wire = [hi, lo, 0xff, 0xce, 0xfc, 0xa9, 0xff, 0xfd];
        let address = MBusAddress::decode(&wire).unwrap();
        assert_eq!(address.encode(), wire);
        if let Some(flag) = address.manufacturer_flag() {
            recognized += 1;
            assert_eq!(u16::from(flag), code);
            assert!(flag.as_str().bytes().all(|c| c.is_ascii_uppercase()));
        }
    }
    assert_eq!(recognized, 26 * 26 * 26);
}

use knx::{
    CodecError,
    structured::*,
    text_time::{DateTime, decode_date_time},
};

#[test]
fn lte_service_selects_status_or_command_and_preserves_fault_precedence() {
    let dpt = LteDpt::from_dpt("204.001").unwrap();
    assert_eq!(dpt.decode_report(&[127, 1]), Ok(LteValue::OutOfService));
    assert_eq!(dpt.decode_command(&[127, 1]), Err(CodecError::OutsideRange));
    assert_eq!(
        dpt.decode_report(&[255, 6]),
        Ok(LteValue::Overridden {
            ticks: -1,
            fault: true
        })
    );
    assert_eq!(dpt.decode_command(&[255, 6]), Ok(LteCommand::SetToDefault));
    for z in 0..=255u8 {
        assert_eq!(
            dpt.decode_report(&[0, z]).is_ok(),
            matches!(z, 0 | 1 | 2 | 4 | 6)
        );
        assert_eq!(dpt.decode_command(&[0, z]).is_ok(), z <= 6);
    }
    for (code, expected) in [
        (0, FaultReason::Unspecified),
        (1, FaultReason::OpenCircuit),
        (2, FaultReason::ShortCircuit),
    ] {
        let mut out = [255; 2];
        dpt.encode_report(LteValue::Fault(expected), &mut out)
            .unwrap();
        assert_eq!(out, [code, 2]);
        assert_eq!(dpt.decode_report(&out), Ok(LteValue::Fault(expected)));
    }
    assert!(dpt.decode_report(&[3, 2]).is_err());
    for (command, bytes) in [
        (LteCommand::Write(-100), [156, 0]),
        (LteCommand::Override(100), [100, 1]),
        (LteCommand::Release, [0, 2]),
        (LteCommand::SetOutOfService, [0, 3]),
        (LteCommand::ResetOutOfService(-1), [255, 4]),
        (LteCommand::AcknowledgeAlarm, [0, 5]),
        (LteCommand::SetToDefault, [0, 6]),
    ] {
        let mut out = [0; 2];
        dpt.encode_command(command, &mut out).unwrap();
        assert_eq!(out, bytes);
        assert_eq!(dpt.decode_command(&out), Ok(command));
    }
    let mut untouched = [22; 2];
    assert!(
        dpt.encode_report(LteValue::Measured(-101), &mut untouched)
            .is_err()
    );
    assert_eq!(untouched, [22; 2]);
    assert!(
        dpt.encode_report(LteValue::OutOfService, &mut [0; 3])
            .is_err()
    );
    assert!(dpt.decode_report(&[0]).is_err());
}

#[test]
fn report_and_command_roundtrip_min_max_for_each_wire_representation() {
    for (id, min, max) in [
        ("202.001", 0, 255),
        ("204.001", -100, 100),
        ("203.007", 0, 65535),
        ("205.017", -32768, 32767),
        ("218.001", i64::from(i32::MIN), i64::from(i32::MAX)),
    ] {
        let dpt = LteDpt::from_dpt(id).unwrap();
        let mut buffer = [0; 5];
        let out = &mut buffer[..dpt.payload_len()];
        for n in [min, 0, max] {
            dpt.encode_report(LteValue::Measured(n), out).unwrap();
            assert_eq!(dpt.decode_report(out), Ok(LteValue::Measured(n)));
            dpt.encode_command(LteCommand::Override(n), out).unwrap();
            assert_eq!(dpt.decode_command(out), Ok(LteCommand::Override(n)));
        }
        for n in [min - 1, max + 1] {
            assert!(dpt.encode_command(LteCommand::Write(n), out).is_err());
        }
    }
}

#[test]
fn independent_status_masks_scene_polarity_and_reserved_values() {
    for code in 0..=255 {
        let flag = GeneralStatus::decode(&[code]);
        assert_eq!(flag.is_ok(), code < 32 && !(code & 1 != 0 && code & 6 != 0));
        if let Ok(value) = flag {
            assert_eq!(value.encode(), Ok([code]));
        }
        let control = DeviceControl::decode(&[code]);
        assert_eq!(control.is_ok(), code < 8);
        if let Ok(value) = control {
            assert_eq!(value.encode(), [code]);
        }
        let scene = SceneConfig::decode(&[code]).unwrap();
        assert_eq!(scene.info.scene.number(), 1 + (code & 63));
        assert_eq!(scene.info.active, code & 64 == 0);
        assert_eq!(scene.storage_enabled, code & 128 == 0);
        assert_eq!(scene.encode(), [code]);
        assert_eq!(SceneInfo::decode(&[code]).is_ok(), code < 128);
        assert_eq!(
            PrioritisedMode::decode(&[code]).unwrap().encode(),
            Ok([if code >= 128 { code & 0xf0 } else { code }])
        );
        assert_eq!(FlaggedScaling::decode(&[88, code]).is_ok(), code <= 1);
    }
    for channel in 0..16 {
        let mut states = [None; 16];
        for state in [Some(false), Some(true)] {
            states[channel] = state;
            let bytes = encode_output_states(states);
            assert_eq!(decode_output_states(&bytes), Ok(states));
        }
    }
    for byte in 0..=255 {
        assert_eq!(
            Version::decode(&[255, byte]).unwrap().is_none(),
            byte == 255
        );
    }
    assert!(
        Version::encode(Some(Version {
            magic: 31,
            version: 31,
            revision: 63
        }))
        .is_err()
    );
    assert_eq!(NextTariff::decode(&[0, 0, 0]), Ok(None));
    assert!(NextTariff::decode(&[0, 0, 255]).is_err());
    assert_eq!(decode_output_states(&[0, 0, 255, 255]), Ok([None; 16]));
}

#[test]
fn alarm_capabilities_and_access_codes() {
    for caps in 0..16 {
        for status in 0..8 {
            let raw = [40, 3, 10, 4, caps, status];
            let val = AlarmInfo::decode(&raw);
            assert_eq!(val.is_ok(), caps & 1 != 0 || status & 2 == 0);
            if let Ok(v) = val {
                assert_eq!(v.can_acknowledge, caps & 1 != 0);
                assert_eq!(v.has_timestamp, caps & 2 != 0);
                assert_eq!(v.has_alarm_text, caps & 4 != 0);
                assert_eq!(v.has_error_code, caps & 8 != 0);
                assert_eq!(v.in_alarm, status & 1 != 0);
                assert_eq!(v.unacknowledged, status & 2 != 0);
                assert_eq!(v.locked, status & 4 != 0);
                assert_eq!(v.encode(), Ok(raw));
            }
        }
    }
    for raw in [
        [0, 0, 1, 6, 0, 0],
        [0, 0, 10, 5, 0, 0],
        [0, 0, 2, 0, 0, 0],
        [0, 0, 1, 0, 16, 0],
        [0, 0, 1, 0, 0, 8],
    ] {
        assert!(AlarmInfo::decode(&raw).is_err());
    }
    for flags in 0..=255 {
        let raw = [0x12, 0x34, 0x56, flags];
        let value = AccessData::decode(&raw).unwrap();
        assert_eq!(value.encode(), Ok(raw));
        assert_eq!(value.read_error, flags & 128 != 0);
        assert_eq!(value.accepted, flags & 64 != 0);
        assert_eq!(value.right_to_left, flags & 32 != 0);
        assert_eq!(value.encrypted, flags & 16 != 0);
        assert_eq!(value.index, flags & 15);
        let other = AccessData {
            digits: [9; 6],
            ..value
        };
        assert_eq!(format!("{value:?}"), format!("{other:?}"));
    }
    for position in 0..6 {
        let mut value = [0; 4];
        value[position / 2] = if position % 2 == 0 { 0xa0 } else { 10 };
        assert!(AccessData::decode(&value).is_err());
    }
    for code in 0..=255 {
        assert_eq!(AlarmReaction::decode(&[code]).is_ok(), code < 3);
        assert_eq!(
            LightApplicationMode::decode(&[code]).is_ok(),
            code <= 2 || code >= 17
        );
        assert_eq!(decode_time_delay_seconds(&[code]).is_ok(), code < 26);
    }
}

#[test]
fn meter_scale_distinguishes_power_energy_and_dimensionless_counts() {
    for code in 0..=255 {
        let expected = matches!(code,0..=31|40..=87|110|128|129|136|137|168|169|176|177|186);
        assert_eq!(MeterScale::from_code(code).is_ok(), expected);
    }
    for (code, expected) in [
        (0, ("Wh", -3)),
        (7, ("Wh", 4)),
        (0x80, ("Wh", 5)),
        (0xa8, ("W", 5)),
        (0xb1, ("J/h", 9)),
        (0xba, ("count", 0)),
    ] {
        assert_eq!(
            MeterScale::from_code(code).unwrap().unit_and_exponent(),
            expected
        );
    }
    let command = MeteringValue::decode_command(&[255, 255, 255, 255, 0x28, 1]).unwrap();
    assert_eq!(command.value, LteCommand::Override(-1));
    assert_eq!(command.encode_command(), Ok([255, 255, 255, 255, 0x28, 1]));
}

#[test]
fn date_validity_stays_independent_from_dated_values() {
    let source = [124, 2, 29, 0, 0, 0, 0x26, 0]; // 2024-02-29, no time or weekday
    let date = decode_date_time(&source).unwrap();
    let DateTime::Value(fields) = date else {
        panic!("expected valid fields")
    };
    assert_eq!(fields.month_day, Some((2, 29)));
    assert_eq!(fields.year, Some(2024));
    assert!(fields.time.is_none());
    for (id, label) in [
        ("265.001", "on"),
        ("265.005", "alarm"),
        ("265.009", "close"),
        ("265.011", "active"),
        ("265.012", "inverted"),
    ] {
        let codec = DatedBinary::from_dpt(id).unwrap();
        let value = Dated {
            at: date,
            value: label,
        };
        let bytes = codec.encode(value).unwrap();
        assert_eq!(&bytes[..8], &source);
        assert_eq!(codec.decode(&bytes), Ok(value));
        let mut invalid = bytes;
        invalid[8] = 2;
        assert!(codec.decode(&invalid).is_err());
    }
    for (id, unit) in [("266.027", "V"), ("266.056", "W"), ("266.080", "VA")] {
        let codec = DatedMeasurement::from_dpt(id).unwrap();
        assert_eq!(codec.unit(), unit);
        let sample = Dated {
            at: date,
            value: 17.5,
        };
        let bytes = codec.encode(sample).unwrap();
        assert_eq!(&bytes[8..], &[65, 140, 0, 0]);
        assert_eq!(codec.decode(&bytes), Ok(sample));
        assert!(
            codec
                .encode(Dated {
                    at: date,
                    value: f32::INFINITY
                })
                .is_err()
        );
    }
    let period = DateTimePeriod {
        start: date,
        stop: DateTime::Fault,
    };
    assert_eq!(
        DateTimePeriod::decode(&period.encode().unwrap()),
        Ok(period)
    );
    let text = Dated {
        at: date,
        value: "kWh / €",
    };
    let mut output = [0; 30];
    let bytes = encode_dated_text(text, &mut output).unwrap();
    assert_eq!(decode_dated_text(bytes), Ok(text));
    assert!(decode_dated_text(&bytes[..bytes.len() - 1]).is_err());
    output[8] = 255;
    assert!(decode_dated_text(&output).is_err());
}

#[test]
fn parameter_periods_and_locale_are_validated_before_use() {
    for codec in [ScalingTimingDpt::Speed, ScalingTimingDpt::StepTime] {
        assert!(codec.decode(&[0, 0, 1]).is_err());
        assert!(codec.decode(&[0, 1, 0]).is_err());
    }
    assert_eq!(normalize_locale(b"DEde"), Ok(*b"deDE"));
    assert_eq!(normalize_locale(b"eNzZ"), Ok(*b"enZZ"));
    assert!(normalize_locale(b"qqXX").is_err());
    assert!(normalize_locale(b"enXY").is_err());
    assert!(Location::decode(&[0x7f, 0x80, 0, 0, 0, 0, 0, 0]).is_err());
}

#[test]
fn hvac_lte_selects_code_domain_only_for_value_bearing_states() {
    let binary = LteDpt::from_dpt("200.100").unwrap();
    assert_eq!(binary.label(0), Some("cooling"));
    assert_eq!(binary.label(1), Some("heating"));
    assert_eq!(binary.label(2), None);
    let mut out = [0; 2];
    binary
        .encode_report(LteValue::Fault(FaultReason::ShortCircuit), &mut out)
        .unwrap();
    assert_eq!(out, [2, 2]);
    assert_eq!(
        binary.decode_report(&out),
        Ok(LteValue::Fault(FaultReason::ShortCircuit))
    );
    assert!(binary.decode_report(&[2, 0]).is_err());
    assert_eq!(binary.decode_command(&[231, 2]), Ok(LteCommand::Release));
    let mode = LteDpt::from_dpt("201.104").unwrap();
    for n in 0..=255 {
        assert_eq!(mode.decode_report(&[n, 0]).is_ok(), n <= 17 || n == 20);
    }
    assert_eq!(mode.label(20), Some("no-demand"));
    for id in ["205.100", "205.101"] {
        let temp = LteDpt::from_dpt(id).unwrap();
        let mut dst = [0; 3];
        assert!(
            temp.encode_report(LteValue::Measured(-13651), &mut dst)
                .is_err()
        );
        temp.encode_report(LteValue::Measured(-13650), &mut dst)
            .unwrap();
        assert_eq!(temp.decode_report(&dst), Ok(LteValue::Measured(-13650)));
        assert_eq!(
            temp.decode_report(&[0x80, 0, 1]),
            Ok(LteValue::OutOfService)
        );
    }
    let sun = LteDpt::from_dpt("203.102").unwrap();
    let mut dst = [0; 3];
    assert!(
        sun.encode_command(LteCommand::Write(28001), &mut dst)
            .is_err()
    );
    sun.encode_command(LteCommand::Write(28000), &mut dst)
        .unwrap();
    assert_eq!(sun.decode_command(&dst), Ok(LteCommand::Write(28000)));
}

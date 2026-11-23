use knx::*;
use std::num::NonZeroU16;

#[test]
fn binary_assignments_and_trigger_semantics() {
    let door = BinaryDpt::from_dpt("1.019").unwrap();
    let actuator = BinaryDpt::from_dpt("1.009").unwrap();
    assert_eq!(door.labels(), ["closed", "open"]);
    assert_eq!(actuator.labels(), ["open", "close"]);
    let trigger = BinaryDpt::from_dpt("1.017").unwrap();
    assert_eq!(trigger.decode(&[0]), trigger.decode(&[1]));
    assert_eq!(trigger.encode("trigger"), Ok([0]));
    for input in [vec![], vec![2], vec![0x81], vec![0, 1]] {
        assert!(trigger.decode(&input).is_err());
    }
    assert!(door.encode("trigger").is_err());
    for invalid in ["1.020", "1.19", " 1.019"] {
        assert!(BinaryDpt::from_dpt(invalid).is_none());
    }
}

#[test]
fn every_float16_word_obeys_the_subtype_floor_and_reserved_code() {
    // Integer hundredths supply an oracle independent of the encoder.
    for (id, minimum) in [
        ("9.001", -27300),
        ("9.002", -67108864),
        ("9.004", 0),
        ("9.027", -45960),
    ] {
        let codec = Float16Dpt::from_dpt(id).unwrap();
        for word in 0..=u16::MAX {
            let sign = if word & 0x8000 == 0 { 0 } else { -2048 };
            let hundredths =
                (sign + i32::from(word % 2048)) * 2_i32.pow(u32::from(word >> 11 & 15));
            let expected = if word == 0x7fff {
                Err(CodecError::InvalidData)
            } else if hundredths < minimum {
                Err(CodecError::OutsideRange)
            } else {
                Ok(f64::from(hundredths) / 100.0)
            };
            assert_eq!(codec.decode(&word.to_be_bytes()), expected);
            if let Ok(value) = expected {
                assert_eq!(codec.decode(&codec.encode(value).unwrap()), Ok(value));
            }
        }
        assert_eq!(codec.encode(670_433.28), Ok([0x7f, 0xfe]));
        assert_eq!(codec.encode(670_433.29), Err(CodecError::OutsideRange));
        assert!(
            codec
                .decode(&codec.encode(f64::from(minimum) / 100.0).unwrap())
                .is_ok()
        );
        assert!(codec.encode(f64::from(minimum) / 100.0 - 0.01).is_err());
        for bad in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(codec.encode(bad), Err(CodecError::NonFinite));
        }
        assert!(matches!(
            codec.decode(&[0]),
            Err(CodecError::PayloadLength { .. })
        ));
    }
}

#[test]
fn float32_edges_and_units() {
    assert!(Float16Dpt::from_dpt("9.012").is_none());
    assert!(Float32Dpt::from_dpt("14.081").is_none());
    for (id, unit) in [
        ("14.000", "m/s²"),
        ("14.019", "A"),
        ("14.056", "W"),
        ("14.080", "VA"),
    ] {
        let dpt = Float32Dpt::from_dpt(id).unwrap();
        assert_eq!(dpt.unit(), unit);
        for n in [
            0.0f32,
            -0.0,
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            f32::MIN,
            f32::MAX,
        ] {
            let bytes = dpt.encode(n).unwrap();
            assert_eq!(dpt.decode(&bytes).unwrap().to_bits(), n.to_bits());
        }
        for n in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            assert_eq!(dpt.encode(n), Err(CodecError::NonFinite));
            assert_eq!(dpt.decode(&n.to_be_bytes()), Err(CodecError::NonFinite));
        }
        assert!(dpt.decode(&[0; 5]).is_err());
    }
}

#[test]
fn integer_domains_keep_all_counts_and_explicit_invalid_values() {
    for value in 0..=u16::MAX {
        let word = value.to_be_bytes();
        assert_eq!(decode_pulse_count_u16(&word), Ok(value));
        assert_eq!(encode_pulse_count_u16(value), word);
        assert_eq!(decode_supply_ma(&word), Ok(NonZeroU16::new(value)));
        assert_eq!(encode_supply_ma(NonZeroU16::new(value)), word);
        let signed = i16::from_be_bytes(word);
        assert_eq!(decode_rotation_degrees(&word), Ok(signed));
        assert_eq!(encode_rotation_degrees(signed), word);
        if signed == i16::MAX {
            assert_eq!(
                decode_percent_hundredths(&word),
                Err(CodecError::InvalidData)
            );
            assert_eq!(
                encode_percent_hundredths(signed),
                Err(CodecError::OutsideRange)
            );
        } else {
            assert_eq!(decode_percent_hundredths(&word), Ok(signed));
            assert_eq!(encode_percent_hundredths(signed), Ok(word));
        }
    }
    for n in [i32::MIN, -1, 0, i32::MAX] {
        assert_eq!(decode_flow_rate_ticks(&encode_flow_rate_ticks(n)), Ok(n));
    }
    for n in [0, u32::MAX] {
        assert_eq!(decode_operating_hours(&encode_operating_hours(n)), Ok(n));
    }
    assert!(decode_supply_ma(&[]).is_err());
    assert!(decode_flow_rate_ticks(&[0; 5]).is_err());
    assert!(decode_percent_hundredths(&[0; 3]).is_err());
}

use knx::*;

#[test]
fn common_unsigned_bytes_cover_every_wire_code() {
    for raw in 0..=u8::MAX {
        let wire = [raw];
        let degrees = decode_angle_degrees(&wire).unwrap();
        assert!((degrees - f64::from(raw) * (360.0 / 255.0)).abs() < 1e-12);
        assert_eq!(encode_angle_degrees(degrees), Ok(wire));
        assert_eq!(decode_decimal_factor(&wire), Ok(raw));
        assert_eq!(encode_decimal_factor(raw), wire);
        assert_eq!(decode_pulse_count(&wire), Ok(raw));
        assert_eq!(encode_pulse_count(raw), wire);
        match raw {
            0 => {
                assert_eq!(Tariff::decode(&wire), Ok(Tariff::UNAVAILABLE));
                assert_eq!(Tariff::UNAVAILABLE.number(), None);
                assert_eq!(Tariff::UNAVAILABLE.encode(), [0]);
                assert_eq!(Tariff::numbered(raw), Err(CodecError::OutsideRange));
            }
            255 => {
                assert_eq!(Tariff::decode(&wire), Err(CodecError::OutsideRange));
                assert_eq!(Tariff::numbered(raw), Err(CodecError::OutsideRange));
            }
            _ => {
                let tariff = Tariff::numbered(raw).unwrap();
                assert_eq!(Tariff::decode(&wire), Ok(tariff));
                assert_eq!(tariff.number(), Some(raw));
                assert_eq!(tariff.encode(), wire);
            }
        }
    }
}

#[test]
fn reject_wrong_width_and_quantize_only_within_the_angle_range() {
    for payload in [&[][..], &[0, 1]] {
        let error = CodecError::PayloadLength {
            expected: 1,
            actual: payload.len(),
        };
        assert_eq!(decode_angle_degrees(payload), Err(error));
        assert_eq!(decode_decimal_factor(payload), Err(error));
        assert_eq!(decode_pulse_count(payload), Err(error));
        assert_eq!(Tariff::decode(payload), Err(error));
    }
    for (angle, raw) in [
        (0.0, 0),
        (60.0, 43),
        (179.99, 127),
        (180.0, 128),
        (360.0, 255),
    ] {
        assert_eq!(encode_angle_degrees(angle), Ok([raw]));
    }
    for angle in [-0.01, 360.01] {
        assert_eq!(encode_angle_degrees(angle), Err(CodecError::OutsideRange));
    }
    for angle in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(encode_angle_degrees(angle), Err(CodecError::NonFinite));
    }
}

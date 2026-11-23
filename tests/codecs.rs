use knx::*;

#[test]
fn switch_and_scaling_reference_values() {
    for (bit, on) in [(0, false), (1, true)] {
        assert_eq!(decode_switch(&[bit]), Ok(on));
        assert_eq!(encode_switch(on), [bit]);
    }
    for bit in 2..=255 {
        assert_eq!(decode_switch(&[bit]), Err(CodecError::OutsideRange));
    }
    for (percent, code) in [(0., 0), (50., 128), (100., 255)] {
        assert_eq!(encode_scaling_percent(percent), Ok([code]));
    }
    for code in 0..=255 {
        assert_eq!(
            encode_scaling_percent(decode_scaling_percent(&[code]).unwrap()),
            Ok([code])
        );
    }
}

#[test]
fn temperature_vectors_and_every_word() {
    for (word, value) in [
        (0x07d0u16, 20.),
        (0x8001, -20.47),
        (0x0c00, 20.48),
        (0x7ffe, 670433.28),
    ] {
        assert_eq!(decode_temperature_celsius(&word.to_be_bytes()), Ok(value));
        assert_eq!(encode_temperature_celsius(value), Ok(word.to_be_bytes()));
    }
    for word in 0..=u16::MAX {
        let exponent = (word / 2048) % 16;
        let signed =
            (i32::from(word % 2048) - 2048 * i32::from(word >= 32768)) * 2i32.pow(exponent.into());
        let decoded = decode_temperature_celsius(&word.to_be_bytes());
        if word == 0x7fff {
            assert_eq!(decoded, Err(CodecError::InvalidData));
        } else if signed < -27300 {
            assert_eq!(decoded, Err(CodecError::OutsideRange));
        } else {
            let value = f64::from(signed) / 100.;
            assert_eq!(decoded, Ok(value));
            assert_eq!(
                decode_temperature_celsius(&encode_temperature_celsius(value).unwrap()),
                decoded
            );
        }
    }
    for (input, quantized) in [(-273., -272.96), (0.125, 0.13), (-0.125, -0.13)] {
        assert_eq!(
            decode_temperature_celsius(&encode_temperature_celsius(input).unwrap()),
            Ok(quantized)
        );
    }
}

#[test]
fn lengths_ranges_energy_and_power() {
    for len in 0..=8 {
        let data = &[0; 8][..len];
        if len != 1 {
            assert_eq!(
                decode_switch(data),
                Err(CodecError::PayloadLength {
                    expected: 1,
                    actual: len
                })
            );
            assert_eq!(
                decode_scaling_percent(data),
                Err(CodecError::PayloadLength {
                    expected: 1,
                    actual: len
                })
            );
        }
        if len != 2 {
            assert_eq!(
                decode_temperature_celsius(data),
                Err(CodecError::PayloadLength {
                    expected: 2,
                    actual: len
                })
            );
        }
        if len != 4 {
            assert_eq!(
                decode_energy_wh(data),
                Err(CodecError::PayloadLength {
                    expected: 4,
                    actual: len
                })
            );
        }
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -274., 670434.] {
        assert!(encode_temperature_celsius(value).is_err());
        assert!(encode_scaling_percent(value).is_err());
    }
    for (value, bytes) in [
        (i32::MIN, [128, 0, 0, 0]),
        (i32::MAX, [127, 255, 255, 255]),
        (-1, [255; 4]),
        (256, [0, 0, 1, 0]),
    ] {
        assert_eq!(decode_energy_wh(&bytes), Ok(value));
        assert_eq!(encode_energy_wh(value), bytes);
    }
    assert_eq!(encode_power_watts(-24000.), Ok([0xc6, 0xbb, 0x80, 0]));
    assert_eq!(encode_power_watts(-0.), Ok([0x80, 0, 0, 0]));
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(encode_power_watts(value), Err(CodecError::NonFinite));
    }
}

#[test]
fn nonfinite_encoder_inputs_are_distinct_from_finite_range_errors() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(encode_scaling_percent(value), Err(CodecError::NonFinite));
        assert_eq!(
            encode_temperature_celsius(value),
            Err(CodecError::NonFinite)
        );
    }
    assert_eq!(encode_scaling_percent(-1.), Err(CodecError::OutsideRange));
    assert_eq!(
        encode_temperature_celsius(-274.),
        Err(CodecError::OutsideRange)
    );
}

#[test]
fn whole_percent_and_scaling_use_the_commissioned_subtype() {
    for raw in 0..=u8::MAX {
        assert_eq!(encode_percent_u8(raw), [raw]);
        assert_eq!(decode_percent_u8(&[raw]), Ok(raw));
    }
    for len in [0, 2, 4] {
        assert_eq!(
            decode_percent_u8(&[0; 4][..len]),
            Err(CodecError::PayloadLength {
                expected: 1,
                actual: len
            })
        );
    }
    assert_eq!(encode_scaling_percent(50.0), Ok([0x80]));
    assert_eq!(encode_percent_u8(50), [0x32]);
    assert_eq!(decode_scaling_percent(&[0xff]), Ok(100.0));
    assert_eq!(decode_percent_u8(&[0xff]), Ok(255));
}

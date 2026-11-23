use knx::*;

#[test]
fn all_relative_command_bytes() {
    let denominators = [1, 2, 4, 8, 16, 32, 64];
    for raw in 0..=u8::MAX {
        let dimming = decode_dimming(&[raw]);
        let blinds = decode_blind_control(&[raw]);
        if raw > 15 {
            assert_eq!(dimming, Err(CodecError::OutsideRange));
            assert_eq!(blinds, Err(CodecError::OutsideRange));
        } else if raw == 0 || raw == 8 {
            assert_eq!(dimming, Ok(Dimming::Stop));
            assert_eq!(blinds, Ok(BlindControl::Stop));
            assert_eq!(encode_dimming(Dimming::Stop), [0]);
            assert_eq!(encode_blind_control(BlindControl::Stop), [0]);
        } else {
            let intervals = denominators[usize::from((raw % 8) - 1)];
            let step = StepSize::from_denominator(intervals).unwrap();
            assert_eq!(step.denominator(), intervals);
            let dim = if raw < 8 {
                Dimming::Decrease(step)
            } else {
                Dimming::Increase(step)
            };
            let movement = if raw < 8 {
                BlindControl::Up(step)
            } else {
                BlindControl::Down(step)
            };
            assert_eq!(dimming, Ok(dim));
            assert_eq!(blinds, Ok(movement));
            assert_eq!(encode_dimming(dim), [raw]);
            assert_eq!(encode_blind_control(movement), [raw]);
        }
        assert_eq!(
            StepSize::from_denominator(raw).is_ok(),
            denominators.contains(&raw)
        );
    }
}

#[test]
fn all_scene_fields_and_display_numbers() {
    for raw in 0..=u8::MAX {
        assert_eq!(SceneNumber::new(raw).is_ok(), (1..=64).contains(&raw));
        let number = SceneNumber::new(raw % 64 + 1).unwrap();
        assert_eq!(number.number(), raw % 64 + 1);
        let decoded = decode_scene_number(&[raw]);
        if raw < 64 {
            assert_eq!(decoded, Ok(number));
            assert_eq!(encode_scene_number(number), [raw]);
        } else {
            assert_eq!(decoded, Err(CodecError::OutsideRange));
        }
        let control = decode_scene_control(&[raw]);
        match raw {
            0..=63 | 128..=191 => {
                let expected = if raw < 64 {
                    SceneControl::Activate(number)
                } else {
                    SceneControl::Learn(number)
                };
                assert_eq!(control, Ok(expected));
                assert_eq!(encode_scene_control(expected), [raw]);
            }
            _ => assert_eq!(control, Err(CodecError::OutsideRange)),
        }
    }
}

#[test]
fn controls_require_one_extracted_byte() {
    for payload in [&[][..], &[0x00, 0x01]] {
        let error = CodecError::PayloadLength {
            expected: 1,
            actual: payload.len(),
        };
        assert_eq!(decode_scene_control(payload), Err(error));
        assert_eq!(decode_scene_number(payload), Err(error));
        assert_eq!(decode_blind_control(payload), Err(error));
        assert_eq!(decode_dimming(payload), Err(error));
    }
}

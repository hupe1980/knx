use knx::*;

#[test]
fn signed_scalars_match_twos_complement_for_every_byte() {
    for raw in 0..=u8::MAX {
        let count = if raw < 128 {
            i16::from(raw)
        } else {
            i16::from(raw) - 256
        };
        let signed = i8::try_from(count).unwrap();
        assert_eq!(decode_percent_i8(&[raw]), Ok(signed));
        assert_eq!(decode_signed_pulse_count(&[raw]), Ok(signed));
        assert_eq!(encode_percent_i8(signed), [raw]);
        assert_eq!(encode_signed_pulse_count(signed), [raw]);
    }
}

#[test]
fn every_flag_combination_and_mode_field_matches_the_wire_table() {
    // A through E are active low. Exercise all 32 independent flag combinations.
    for flags in 0u8..32 {
        let set_flags = [16, 8, 4, 2, 1].map(|mask| flags & mask == 0);
        for field in 0..8 {
            let wire = [(flags << 3) | field];
            let mode = match field {
                1 => StatusMode::Mode0,
                2 => StatusMode::Mode1,
                4 => StatusMode::Mode2,
                _ => {
                    assert_eq!(StatusWithMode::decode(&wire), Err(CodecError::OutsideRange));
                    continue;
                }
            };
            let expected = StatusWithMode { set_flags, mode };
            assert_eq!(StatusWithMode::decode(&wire), Ok(expected));
            assert_eq!(expected.encode(), wire);
        }
    }
}

#[test]
fn signed_and_status_payloads_require_exactly_one_byte() {
    for payload in [&[][..], &[1, 2]] {
        let error = CodecError::PayloadLength {
            expected: 1,
            actual: payload.len(),
        };
        assert_eq!(StatusWithMode::decode(payload), Err(error));
        assert_eq!(decode_percent_i8(payload), Err(error));
        assert_eq!(decode_signed_pulse_count(payload), Err(error));
    }
}

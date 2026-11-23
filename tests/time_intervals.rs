use knx::{CodecError, TimeIntervalDpt};

#[test]
fn every_interval_word_round_trips_in_its_commissioned_time_base() {
    for (suffix, scale) in [
        ("002", 1),
        ("003", 10),
        ("004", 100),
        ("005", 1000),
        ("006", 60_000),
        ("007", 3_600_000),
    ] {
        for family in ["7", "8"] {
            let dpt = TimeIntervalDpt::from_dpt(&format!("{family}.{suffix}")).unwrap();
            assert_eq!(dpt.resolution_ms(), scale);
            for raw in 0..=u16::MAX {
                let mut ticks = i64::from(raw);
                if family == "8" && raw >= 0x8000 {
                    ticks -= 65536;
                }
                let ms = ticks * scale;
                let wire = raw.to_be_bytes();
                assert_eq!(dpt.decode_ms(&wire), Ok(ms));
                assert_eq!(dpt.encode_ms(ms), Ok(wire));
            }
            let (min, max) = if family == "7" {
                (0, 65535)
            } else {
                (-32768, 32767)
            };
            assert_eq!(dpt.range_ms(), min * scale..=max * scale);
            for invalid in [min * scale - 1, max * scale + 1, i64::MIN, i64::MAX] {
                assert_eq!(dpt.encode_ms(invalid), Err(CodecError::OutsideRange));
            }
            if scale > 1 {
                assert_eq!(
                    dpt.encode_ms(scale + 1),
                    Err(CodecError::InexactInterval {
                        resolution_ms: scale
                    })
                );
                if family == "8" {
                    assert_eq!(
                        dpt.encode_ms(-1),
                        Err(CodecError::InexactInterval {
                            resolution_ms: scale
                        })
                    );
                }
            }
            for payload in [&[][..], &[0], &[0, 0, 0]] {
                assert_eq!(
                    dpt.decode_ms(payload),
                    Err(CodecError::PayloadLength {
                        expected: 2,
                        actual: payload.len()
                    })
                );
            }
        }
    }
    for id in ["7.001", "8.008", "9.005", "7.2", "7.002.0", "7"] {
        assert_eq!(TimeIntervalDpt::from_dpt(id), None);
    }
}

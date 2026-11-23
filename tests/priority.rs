use knx::{CodecError, PriorityControl, PriorityDpt};

#[test]
fn all_priority_subtypes_validate_every_possible_byte() {
    for (id, dpt, zero, one) in [
        ("2.001", PriorityDpt::Switch, "off", "on"),
        ("2.002", PriorityDpt::Boolean, "false", "true"),
        ("2.003", PriorityDpt::Enable, "disable", "enable"),
        ("2.004", PriorityDpt::Ramp, "no-ramp", "ramp"),
        ("2.005", PriorityDpt::Alarm, "no-alarm", "alarm"),
        ("2.006", PriorityDpt::BinaryValue, "low", "high"),
        ("2.007", PriorityDpt::Step, "decrease", "increase"),
        ("2.008", PriorityDpt::UpDown, "up", "down"),
        ("2.009", PriorityDpt::OpenClose, "open", "close"),
        ("2.010", PriorityDpt::Start, "stop", "start"),
        ("2.011", PriorityDpt::State, "inactive", "active"),
        ("2.012", PriorityDpt::Invert, "normal", "inverted"),
    ] {
        assert_eq!(PriorityDpt::from_dpt(id), Some(dpt));
        assert_eq!(dpt.labels(), (zero, one));
        for raw in 0..=u8::MAX {
            let expected = match raw {
                0 | 1 => Ok(PriorityControl::Release),
                2 => Ok(PriorityControl::Set(false)),
                3 => Ok(PriorityControl::Set(true)),
                _ => Err(CodecError::OutsideRange),
            };
            assert_eq!(dpt.decode(&[raw]), expected);
            if let Ok(command) = expected {
                assert_eq!(dpt.encode(command), [if raw == 1 { 0 } else { raw }]);
            }
        }
        for payload in [&[][..], &[0, 1]] {
            assert_eq!(
                dpt.decode(payload),
                Err(CodecError::PayloadLength {
                    expected: 1,
                    actual: payload.len()
                })
            );
        }
    }
    for id in ["2", "2.01", "2.000", "2.013", "1.001"] {
        assert_eq!(PriorityDpt::from_dpt(id), None);
    }
}

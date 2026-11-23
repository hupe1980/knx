use knx::{MeterError, PowerMeter, PowerReading};

#[test]
fn rejects_invalid_bindings() {
    assert_eq!(
        PowerMeter::new("west", "0", 1.).err(),
        Some(MeterError::InvalidGroup)
    );
    assert_eq!(
        PowerMeter::new(" ", "1", 1.).err(),
        Some(MeterError::InvalidSite)
    );
    for max in [0., -1., f32::NAN, f32::INFINITY] {
        assert_eq!(
            PowerMeter::new("west", "1", max).err(),
            Some(MeterError::InvalidRange)
        );
    }
}

#[test]
fn rejected_samples_preserve_the_last_observation() {
    use MeterError::*;
    let zero = 0f32.to_be_bytes();
    let mut meter = PowerMeter::new("west", "2/1/3", 60000.).unwrap();
    assert_eq!(
        meter.observe("west", "2/259", "14.056", &zero, Some(1), 1),
        Ok(PowerReading {
            watts: 0.,
            observed_at: 1
        })
    );
    for (site, group, dpt, payload, at, reason) in [
        ("east", "2/1/3", "14.056", &zero[..], Some(2), WrongBinding),
        (
            "west",
            "2/1/3",
            "14.056",
            &[0xc6, 0xbb, 0x80, 0x00],
            Some(2),
            OutsideRange,
        ),
        ("west", "2/1/4", "14.056", &zero, Some(2), WrongBinding),
        ("west", "2/1/3", "9.024", &zero, Some(2), WrongDpt),
        (
            "west",
            "2/1/3",
            "14.056",
            &zero[..3],
            Some(2),
            PayloadLength,
        ),
        (
            "west",
            "2/1/3",
            "14.056",
            &f32::NAN.to_be_bytes(),
            Some(2),
            OutsideRange,
        ),
        (
            "west",
            "2/1/3",
            "14.056",
            &60001f32.to_be_bytes(),
            Some(2),
            OutsideRange,
        ),
        ("west", "2/1/3", "14.056", &zero, None, MissingTime),
        ("west", "2/1/3", "14.056", &zero, Some(4), FutureTime),
        ("west", "2/1/3", "14.056", &zero, Some(1), OutOfOrder),
        ("west", "2/1/3", "14.056", &zero, Some(0), OutOfOrder),
    ] {
        assert_eq!(meter.observe(site, group, dpt, payload, at, 3), Err(reason));
    }
    assert_eq!(
        meter.observe("west", "2/1/3", "14.056", &zero, Some(2), 3),
        Ok(PowerReading {
            watts: 0.,
            observed_at: 2
        })
    );
}

use knx::{CodecError, HvacMode, Tariff, TariffEnergy};

#[test]
fn tariff_and_energy_validity_are_independent() {
    for flags in 0..=255 {
        let decoded = TariffEnergy::decode(&[255, 255, 255, 254, 4, flags]);
        if flags & 0xfc != 0 {
            assert_eq!(decoded, Err(CodecError::InvalidData));
            continue;
        }
        let reading = decoded.unwrap();
        assert_eq!(reading.watt_hours, (flags & 2 == 0).then_some(-2));
        assert_eq!(
            reading.tariff,
            (flags & 1 == 0).then(|| Tariff::numbered(4).unwrap())
        );
        assert_eq!(reading.encode()[5], flags & 3);
        assert_eq!(TariffEnergy::decode(&reading.encode()), Ok(reading));
    }
    assert_eq!(
        TariffEnergy::decode(&[0, 0, 0, 0, 255, 0]),
        Err(CodecError::OutsideRange)
    );
    assert_eq!(
        TariffEnergy::decode(&[0, 0, 0, 0, 255, 1]).unwrap().tariff,
        None
    );
    assert_eq!(
        TariffEnergy::decode(&[0; 6]).unwrap().tariff,
        Some(Tariff::UNAVAILABLE)
    );
    for wh in [i32::MIN, i32::MAX] {
        let value = TariffEnergy {
            watt_hours: Some(wh),
            tariff: None,
        };
        assert_eq!(TariffEnergy::decode(&value.encode()), Ok(value));
    }
    for width in [0, 5, 7] {
        assert_eq!(
            TariffEnergy::decode(&vec![0; width]),
            Err(CodecError::PayloadLength {
                expected: 6,
                actual: width
            })
        );
    }
}

#[test]
fn room_modes_cover_only_the_five_assigned_codes() {
    for (code, mode) in [
        HvacMode::Auto,
        HvacMode::Comfort,
        HvacMode::Standby,
        HvacMode::Economy,
        HvacMode::BuildingProtection,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(mode.encode(), [code as u8]);
        assert_eq!(HvacMode::decode(&[code as u8]), Ok(mode));
    }
    for code in 5..=255 {
        assert_eq!(HvacMode::decode(&[code]), Err(CodecError::OutsideRange));
    }
    assert_eq!(
        HvacMode::decode(&[0, 0]),
        Err(CodecError::PayloadLength {
            expected: 1,
            actual: 2
        })
    );
}

#[test]
fn faulted_timestamp_preserves_register() {
    use knx::{
        DatedTariffEnergy,
        text_time::{CalendarFields, DateTime},
    };
    let value = DatedTariffEnergy {
        timestamp: DateTime::Fault,
        register: TariffEnergy {
            watt_hours: Some(300),
            tariff: None,
        },
    };
    let wire = [0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 1, 44, 0, 1];
    assert_eq!(value.encode(), Ok(wire));
    assert_eq!(DatedTariffEnergy::decode(&wire), Ok(value));
    assert!(DatedTariffEnergy::decode(&wire[1..]).is_err());
    let unavailable = DateTime::Value(CalendarFields::default());
    let value = DatedTariffEnergy {
        timestamp: unavailable,
        ..value
    };
    assert_eq!(
        DatedTariffEnergy::decode(&value.encode().unwrap()),
        Ok(value)
    );
    let reliable = CalendarFields {
        reliable_source: true,
        ..Default::default()
    };
    assert!(
        DatedTariffEnergy {
            timestamp: DateTime::Value(reliable),
            ..value
        }
        .encode()
        .is_err()
    );
}

#[test]
fn lighting_status_flags_and_actuator_constraints() {
    use knx::{LightingActuator::*, LightingStatus};
    for flags in 0..=255 {
        let status = LightingStatus::decode(Dimmer, &[64, flags]).unwrap();
        assert_eq!(status.failure, flags & 128 != 0);
        assert_eq!(status.local_override, flags & 64 != 0);
        assert_eq!(status.dimming, flags & 32 != 0);
        assert_eq!(status.staircase, flags & 16 != 0);
        assert_eq!(status.night_mode, flags & 8 != 0);
        assert_eq!(status.forced, flags & 4 != 0);
        assert_eq!(status.locked, flags & 2 != 0);
        assert_eq!(status.level, (flags & 1 != 0).then_some(64));
        assert_eq!(
            status.encode(Dimmer),
            Ok([if flags & 1 != 0 { 64 } else { 0 }, flags])
        );
        assert_eq!(status.encode(Switch).is_err(), flags & 33 != 0);
    }
    assert!(LightingStatus::decode(Switch, &[255, 1]).is_ok());
    assert!(LightingStatus::decode(Switch, &[255, 33]).is_err());
}

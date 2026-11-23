use knx::hvac::*;
#[test]
fn assigned_modes_preserve_codes_and_reject_reserved_slots() {
    macro_rules! codes {
        ($name:ident, $values:expr) => {{
            let assigned = $values;
            assert!($name::decode(&[]).is_err());
            for raw in 0..=255 {
                let result = $name::decode(&[raw]);
                assert_eq!(result.is_ok(), assigned.contains(&raw));
                if let Ok(value) = result {
                    assert_eq!(value.encode(), [raw]);
                }
            }
        }};
    }
    codes!(HeatCool, [0, 1]);
    codes!(Fuel, [0, 1, 2, 3]);
    codes!(Burner, [1, 2, 3]);
    codes!(HotWaterMode, [0, 1, 2, 3, 4]);
    codes!(LoadPriority, [0, 1, 2]);
    codes!(
        ControllerMode,
        [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20
        ]
    );
    codes!(EmergencyMode, [0, 1, 2, 3, 4, 5]);
    codes!(Changeover, [0, 1, 2]);
    codes!(ValveMode, [1, 2, 3, 4, 5]);
    codes!(DamperMode, [1, 2, 3, 4]);
    codes!(HeaterMode, [1, 2, 3]);
    codes!(FanMode, [0, 1, 2]);
    codes!(ControllerRole, [0, 1, 2]);
    codes!(RoomSetpoint, [0, 1, 2]);
    codes!(
        MeterDevice,
        [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 32, 33, 40, 41, 255
        ]
    );
    codes!(HumidityMode, [0, 1, 2]);
    codes!(DamperType, [1, 2]);
    codes!(BackupMode, [0, 1]);
    codes!(Synchronization, [0, 1, 2]);
    codes!(PushButtonAction, [0, 1, 2, 3]);
    codes!(PushButtonParameter, [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn delay_and_mode_validity_depend_on_subtype() {
    use knx::HvacMode;
    assert_eq!(
        ModeDelay::<HvacMode>::decode(&[0, 0, 3]).unwrap().mode,
        Some(HvacMode::Economy)
    );
    assert_eq!(
        ModeDelay::<HvacMode>::decode(&[1, 2, 0]).unwrap().mode,
        None
    );
    assert!(
        ModeDelay::<HvacMode> {
            minutes: None,
            mode: Some(HvacMode::Auto)
        }
        .encode()
        .is_err()
    );
    assert_eq!(Scheduled::<Occupancy>::decode(&[0, 0, 255]), Ok(None));
    assert!(Scheduled::<Occupancy>::decode(&[0, 1, 255]).is_err());
    assert_eq!(
        Scheduled::<TemperatureTicks>::decode(&[0, 0, 128, 0]),
        Ok(None)
    );
    assert!(Scheduled::<TemperatureTicks>::decode(&[0, 1, 128, 0]).is_err());
}
#[test]
fn setpoint_ranges_and_independent_invalid_fields() {
    assert!(RoomTemperatures([-13651, 0, 0]).encode().is_err());
    assert_eq!(
        RoomTemperatures([-13650, 0, 0]).encode(),
        Ok([202, 174, 0, 0, 0, 0])
    );
    assert!(RoomShifts([i16::MIN, 0, 0]).encode().is_err());
    assert!(RoomShifts([-32767, 0, 32767]).encode().is_ok());
    for i in 0..3 {
        let mut fields = [Some(-1.0); 3];
        fields[i] = None;
        let payload = FloatSetpoints::Celsius.encode(fields).unwrap();
        assert_eq!(FloatSetpoints::Celsius.decode(&payload), Ok(fields));
    }
    assert!(FloatSetpoints::Celsius.encode([Some(-274.0); 3]).is_err());
    assert!(
        FloatSetpoints::KelvinShift
            .encode([Some(-274.0); 3])
            .is_ok()
    );
    assert!(AirDemand::decode(&[155, 1, 0]).is_err());
    assert!(WaterDemand::decode(&[101, 1]).is_err());
    assert!(SupplyAirSetpoints::decode(&[0, 0, 0, 0, 12, 0]).is_err());
}

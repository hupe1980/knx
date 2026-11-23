use knx::{
    CodecError,
    hvac::{Burner, TemperatureTicks},
    hvac_flags::FuelTypes,
    hvac_reports::*,
};

#[test]
fn reported_percent_is_bounded_only_when_present() {
    for n in 0..=255 {
        let invalid = BurnerStatus::decode(&[n, 2]).unwrap();
        assert_eq!(invalid.power_percent, None);
        assert!(invalid.fault);
        assert_eq!(BurnerStatus::decode(&[n, 1]).is_ok(), n <= 100);
        assert_eq!(ActuatorStatus::decode(&[n, 1]).is_ok(), n <= 100);
        assert_eq!(
            LockSignal {
                reduction_percent: Some(n),
                critical: false
            }
            .encode()
            .is_ok(),
            n <= 100
        );
        assert_eq!(
            BoilerDemand {
                demand_percent: n,
                ..Default::default()
            }
            .encode()
            .is_ok(),
            n <= 100
        );
    }
    assert!(ActuatorDemand::decode(&[0; 3]).is_err());
}

#[test]
fn flow_temperature_uses_absolute_bounds_and_preserves_fault_without_temperature() {
    for ticks in [i16::MIN, -13651, -13650, 0, 32767] {
        let [hi, lo] = ticks.to_be_bytes();
        assert_eq!(RoomDemand::decode(&[hi, lo, 1]).is_ok(), ticks >= -13650);
        assert_eq!(
            HeatManagerStatus::decode(&[hi, lo, 2])
                .unwrap()
                .flow_temperature,
            None
        );
        if let Ok(t) = TemperatureTicks::new(ticks) {
            let status = WaterControllerStatus {
                water_temperature: Some(t),
                fault: true,
                controller_running: true,
            };
            assert_eq!(WaterControllerStatus::decode(&status.encode()), Ok(status));
        }
    }
    assert!(CoolingManagerStatus::decode(&[0; 2]).is_err());
}

#[test]
fn hot_water_dependency_survives_all_flow_flag_combinations() {
    for flags in 0..=4095u16 {
        let [hi, lo] = flags.to_be_bytes();
        let raw = [0, 0, hi, lo];
        let value = FlowDemand::decode(&raw);
        if flags & 0x800 != 0 && flags & 0x20 == 0 {
            assert_eq!(value, Err(CodecError::OutsideRange));
        } else {
            assert_eq!(value.unwrap().encode(), Ok(raw));
        }
    }
    assert!(
        FlowDemand {
            hot_water_legionella: true,
            ..Default::default()
        }
        .encode()
        .is_err()
    );
}

#[test]
fn dual_status_fields_are_independently_usable() {
    let mut raw = [0x80, 0, 255, 0, 0x4c];
    let v = ChillerStatus::decode(&raw).unwrap();
    assert!(v.running && v.fault && v.request_next_chiller);
    assert_eq!((v.temperature, v.power_percent), (None, None));
    raw[4] |= 2;
    assert!(ChillerStatus::decode(&raw).is_err());
    raw[2] = 10;
    assert_eq!(ChillerStatus::decode(&raw).unwrap().power_percent, Some(10));
    raw[4] |= 1;
    assert!(ChillerStatus::decode(&raw).is_err());
    assert!(BoilerStatus::decode(&[0; 4]).is_err());
    assert!(HeatPowerDemand::decode(&[0, 0, 101, 0]).is_err());
    assert!(
        CoolPowerDemand::decode(&[0x80, 0, 101, 4])
            .unwrap()
            .pump_enabled
    );
    assert!(CoolPowerDemand::decode(&[0, 0, 101, 2]).is_err());
}

#[test]
fn heat_producer_spec_checks_burner_and_base_stage() {
    let mut spec = HeatProducerSpec {
        nominal_kw: u16::MAX,
        base_stage_percent: 100,
        burner: Burner::SingleStage,
        fuels: FuelTypes {
            gas: true,
            ..Default::default()
        },
    };
    assert_eq!(HeatProducerSpec::decode(&spec.encode().unwrap()), Ok(spec));
    spec.base_stage_percent = 99;
    assert!(spec.encode().is_err());
    spec.burner = Burner::Modulating;
    assert!(spec.encode().is_ok());
    assert!(HeatProducerSpec::decode(&[0, 1, 100, 4, 0]).is_err());
}

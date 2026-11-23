use knx::{CodecError, domain::*, text_time::DateTime};

#[test]
fn metering_is_bound_to_wire_width_clock_and_medium() {
    assert_eq!(
        PlcTechnology::G3.decode(&[2]),
        Err(CodecError::OutsideRange)
    );
    assert_eq!(
        PlcTechnology::Sfsk.encode(PlcStatus::Associated),
        Err(CodecError::OutsideRange)
    );
    assert!(BreakerStatus::decode_dated(&[0; 8]).is_err());
    assert!(EnergyDirection::decode(&[2]).is_err());
    assert!(MeterRole::decode_dated(&[0, 0, 0, 0, 0, 0, 128, 0, 2]).is_err());
    assert_eq!(
        BreakerValveState::decode(&[254]),
        Err(CodecError::OutsideRange)
    );
    for kind in [
        PhaseQuantity::Current,
        PhaseQuantity::Potential,
        PhaseQuantity::ApparentPower,
    ] {
        for i in 0..3 {
            let mut data = [0.; 3];
            data[i] = f32::INFINITY;
            assert_eq!(kind.encode(data), Err(CodecError::NonFinite));
        }
        assert!(kind.decode(&[0; 11]).is_err());
        assert!(kind.decode_dated(&[0; 19]).is_err());
        let mut invalid = kind.encode_dated(DateTime::Fault, [0.; 3]).unwrap();
        invalid[12..16].copy_from_slice(&f32::NAN.to_be_bytes());
        assert_eq!(kind.decode_dated(&invalid), Err(CodecError::NonFinite));
    }
    for x in 0..=255 {
        assert_eq!(encode_contacts(decode_contacts(&[x]).unwrap()), [x]);
    }
    for x in 0..8 {
        assert_eq!(
            encode_phase_presence(decode_phase_presence(&[x]).unwrap()),
            [x]
        );
    }
}

#[test]
fn blinds_and_lights_keep_validity_and_control_semantics() {
    for x in 0..4 {
        let p = CombinedPosition::decode(&[123, 98, x]).unwrap();
        assert_eq!(p.height, (x & 1 != 0).then_some(123));
        assert_eq!(p.slats, (x & 2 != 0).then_some(98));
        assert_eq!(CombinedPosition::decode(&p.encode()).unwrap(), p);
    }
    for x in 0..=u16::MAX {
        let [h, l] = x.to_be_bytes();
        let s = BlindStatus::decode(&[0, 0, h, l]).unwrap();
        let encoded = s.encode();
        assert_eq!(u16::from_be_bytes([encoded[2], encoded[3]]), x & !0x3800);
    }
    for mask in 0..16 {
        let color = Rgbw::decode(&[10, 20, 30, 40, 0, mask]).unwrap();
        assert_eq!(color.red, (mask & 8 != 0).then_some(10));
        assert_eq!(color.white, (mask & 1 != 0).then_some(40));
        assert_eq!(Rgbw::decode(&color.encode()), Ok(color));
    }
    assert!(Rgb::decode(&[0; 4]).is_err());
    assert!(LightLockBehavior::decode(&[7]).is_err());
    assert!(ConverterTestControl::decode(&[0]).is_err());
    assert!(BlindInputMode::decode(&[0]).is_err());
    assert!(decode_fade_tenths(&[16]).is_err());
    assert!(encode_fade_tenths(8).is_err());
}

#[test]
fn weather_checks_valid_fields_and_wire_order_after_quantization() {
    let temp = WeatherDpt::from_dpt("273.001").unwrap();
    assert_eq!(
        temp.decode(&[0, 0, 0, 255, 127, 255, 127, 255]),
        Ok(Forecast::default())
    );
    for bad in [
        [8, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 0, 127, 255, 0, 0],
        [3, 0, 0, 0, 0, 1, 0, 2],
        [3, 0, 0, 0, 0, 2, 0, 2],
    ] {
        assert!(temp.decode(&bad).is_err());
    }
    let narrow = Forecast {
        minimum: Some(1.),
        maximum: Some(1.000001),
        ..Default::default()
    };
    assert!(temp.encode(narrow).is_err());
    assert!(encode_wind_forecast(narrow).is_err());
    assert!(decode_wind_forecast(&[3, 0, 0, 0, 5, 6]).is_err());
    assert_eq!(WeatherDpt::from_dpt("273.004").unwrap().unit(), "%");
}

#[test]
fn colour_masks_quantization_and_relative_stop() {
    let color = XyColor {
        coordinates: Some((0.5, 1.0)),
        brightness: None,
    };
    assert_eq!(color.encode(), Ok([128, 0, 255, 255, 0, 2]));
    let canonical = XyColor::decode(&color.encode().unwrap()).unwrap();
    assert_eq!(
        XyColor::decode_transition(&color.encode_transition(23).unwrap()),
        Ok((23, canonical))
    );
    for x in [-0.1, 1.1, f64::NAN] {
        assert!(
            XyColor {
                coordinates: Some((x, 0.0)),
                brightness: None
            }
            .encode()
            .is_err()
        );
    }
    assert_eq!(
        WhiteTransition::decode(&[0, 9, 0, 42, 255, 0]),
        Ok(WhiteTransition::default())
    );
    assert!(WhiteControl::decode(&[0, 0]).is_err());
    assert_eq!(
        WhiteControl::decode(&[8, 15, 2]),
        Ok(WhiteControl {
            temperature: Some(knx::Dimming::Stop),
            brightness: None
        })
    );
    for code in 0..16 {
        let steps = decode_rgb_steps(&[code; 3]).unwrap();
        assert_eq!(
            encode_rgb_steps(steps),
            [if code == 8 { 0 } else { code }; 3]
        );
    }
    assert_eq!(knx::decode_delta_litres(&(-1i32).to_be_bytes()), Ok(-1));
    assert_eq!(knx::decode_volume_litres(&[255; 4]), Ok(u32::MAX));
    assert_eq!(knx::decode_colour_kelvin(&[255; 2]), Ok(u16::MAX));
    assert_eq!(
        knx::Float32Dpt::from_dpt("14.1201")
            .unwrap()
            .encode(f32::NAN),
        Err(CodecError::NonFinite)
    );
}

#[test]
fn standard_dali_diagnostics_keep_unknown_and_reserved_distinct() {
    use knx::domain::dali::*;
    for i in 0..=255 {
        let diag = DaliDiagnostic::decode(&[i]).unwrap();
        assert_eq!(diag.encode(), Ok([i]));
        assert_eq!(LightFaults::decode(&[i]).unwrap().encode(), [i & 127]);
    }
    assert!(GearDiagnostic::decode(&[0, 0x50]).is_err());
    assert!(
        GearDiagnostic {
            address: DaliAddress::Group(16),
            read_request: true,
            lamp_failure: false,
            ballast_failure: false,
            converter_error: false
        }
        .encode()
        .is_err()
    );
    assert!(
        BatteryInfo {
            charge: Some(255),
            ..Default::default()
        }
        .encode()
        .is_err()
    );
    assert_eq!(
        ConverterFixed::decode(&[2, 1, 2, 3, 4]),
        Ok(ConverterInfo::Invalid)
    );
    assert!(ConverterRuntime::decode(&[1, 0, 0, 0, 0, 0, 64]).is_err());
    assert!(ConverterRuntime::decode(&[3; 7]).is_err());
    assert!(ConverterResults::decode(&[0, 0, 0, 1, 255, 255]).is_err());
    assert!(ConverterResults::decode(&[0x60, 0, 0, 0, 0, 255]).is_err());
    assert!(ConverterResults::decode(&[0, 0, 0xc0, 0, 0, 255]).is_err());
    assert!(ConverterStatus::decode(&[0xa0, 0]).is_err());
    assert!(ConverterStatus::decode(&[0, 3]).is_err());
    assert!(ConverterTestInfo::decode(&[0; 11]).is_err());
    assert!(TariffDayItem::decode(&[0, 0, 0, 0, 255, 0, 0]).is_err());
    assert!(ErlStatus::decode(&[0; 3]).is_err());
}

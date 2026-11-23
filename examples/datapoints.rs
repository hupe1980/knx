use knx::*;

fn main() -> Result<(), CodecError> {
    assert!(decode_switch(&encode_switch(true))?);
    let level = decode_scaling_percent(&encode_scaling_percent(50.0)?)?;
    assert_eq!(decode_percent_u8(&encode_percent_u8(255))?, 255);
    assert_eq!(PriorityDpt::Switch.decode(&[1])?, PriorityControl::Release);
    assert_eq!(encode_angle_degrees(180.0)?, [0x80]);
    assert_eq!(Tariff::decode(&[0])?.number(), None);
    assert_eq!(decode_signed_pulse_count(&[0xff])?, -1);
    assert_eq!(StatusWithMode::decode(&[0x52])?.mode, StatusMode::Mode1);
    let delta = TimeIntervalDpt::from_dpt("8.005").unwrap();
    assert_eq!(delta.decode_ms(&[0xff, 0xfe])?, -2000);
    let quarter = StepSize::from_denominator(4)?;
    assert_eq!(encode_dimming(Dimming::Increase(quarter)), [0x0b]);
    assert_eq!(decode_blind_control(&[0x03])?, BlindControl::Up(quarter));
    let scene = SceneNumber::new(64)?;
    assert_eq!(encode_scene_number(scene), [0x3f]);
    assert_eq!(decode_scene_control(&[0xbf])?, SceneControl::Learn(scene));
    let room = decode_temperature_celsius(&encode_temperature_celsius(20.0)?)?;
    let energy = decode_energy_wh(&encode_energy_wh(-1))?;
    let register = Energy64::ActiveWh;
    let total = register.decode(&[0, 32, 0, 0, 0, 0, 0, 1])?;
    assert_eq!(total, 9_007_199_254_740_993);
    let power = decode_power_watts(&encode_power_watts(-24_000.0)?)?;
    println!("{level:.1}% · {room} °C · {energy} Wh · {power} W · register {total} Wh");
    Ok(())
}

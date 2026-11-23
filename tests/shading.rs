use knx::shading::{Direction, SunblindActuator};

#[test]
fn mandatory_inputs_match_xknx_cover() {
    let travel = "1/2/3".parse().unwrap();
    let step = "1/2/4".parse().unwrap();
    assert!(SunblindActuator::new(travel, travel).is_err());
    let blind = SunblindActuator::new(travel, step).unwrap();
    // XKNX 4.1.1 Cover.set_{up,down,short_up,short_down}, source 1.1.1.
    for (request, group, apci) in [
        (blind.travel(0x1101, Direction::Up), 3, 0x80),
        (blind.travel(0x1101, Direction::Down), 3, 0x81),
        (blind.stop_or_step(0x1101, Direction::Up), 4, 0x80),
        (blind.stop_or_step(0x1101, Direction::Down), 4, 0x81),
    ] {
        let mut bytes = [0; 11];
        assert_eq!(request.encode(&mut bytes).unwrap(), bytes.len());
        assert_eq!(bytes, [0x11, 0, 0xbc, 0xe0, 0x11, 1, 10, group, 1, 0, apci]);
        assert!(request.indicated_value().is_none());
    }
}

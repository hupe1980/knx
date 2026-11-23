use knx::{GroupAddress, InvalidGroupAddress, PowerMeter};

#[test]
fn all_group_address_styles_round_trip() {
    for raw in 1..=u16::MAX {
        let address = GroupAddress::try_from(raw).unwrap();
        let canonical = format!("{}/{}/{}", raw >> 11, (raw >> 8) & 7, raw & 255);
        assert_eq!(address.to_string(), canonical);
        assert_eq!(u16::from(address), raw);
        for text in [
            raw.to_string(),
            format!("{}/{}", raw >> 11, raw & 2047),
            canonical,
        ] {
            assert_eq!(text.parse(), Ok(address));
        }
    }
}

#[test]
fn rejects_reserved_and_malformed_destinations() {
    assert_eq!(GroupAddress::try_from(0), Err(InvalidGroupAddress));
    for text in [
        "0", "0/0", "0/0/0", "", "01", "/1", "1/", "1//1", "1/0/0/0", "32/0", "1/2048", "1/8/0",
        "1/0/256", "65536", "+1", " 1", "1\n", "１",
    ] {
        assert_eq!(text.parse::<GroupAddress>(), Err(InvalidGroupAddress));
    }
}

#[test]
fn numeric_transport_address_matches_configured_binding() {
    let configured: GroupAddress = "2/1/3".parse().unwrap();
    let mut meter = PowerMeter::new("west", configured, 60000.).unwrap();
    let reading = meter
        .observe(
            "west",
            4355u16,
            "14.056",
            &24000f32.to_be_bytes(),
            Some(1),
            2,
        )
        .unwrap();
    assert_eq!(reading.watts, 24000.);
}

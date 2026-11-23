use knx::telegram::{GroupData, GroupService, GroupTelegram, LinkPrimitive, TelegramError};
use knx::{decode_switch, decode_temperature_celsius};

const SWITCH: [u8; 11] = [0x29, 0, 0xbc, 0xe0, 0xff, 0xff, 0x23, 2, 1, 0, 0x81];

#[test]
fn manufacturer_vectors_distinguish_link_evidence_and_service_data() {
    // Weinzierl BAOS Starter Kit guide §5.2.3, pp. 48–49.
    let read = [0x11, 0, 0x9c, 0xe0, 0, 0, 0x0a, 1, 1, 0, 0];
    let response = [0x29, 0, 0xbc, 0xe0, 0x11, 1, 0x0a, 1, 1, 0, 0x41];
    let confirmation = [0x2e, 0, 0xbc, 0xe0, 0xff, 0xff, 0x23, 2, 1, 0, 0x81];
    let two_octets = [
        0x29, 0, 0xbc, 0xe0, 0xff, 0xff, 0x7f, 0xfe, 3, 0, 0x80, 0xff, 0xfe,
    ];
    for frame in [&read[..], &response, &SWITCH, &confirmation, &two_octets] {
        let telegram = GroupTelegram::parse(frame).unwrap();
        let mut bytes = [0; 25];
        let count = telegram.encode(&mut bytes).unwrap();
        assert_eq!(&bytes[..count], frame);
    }
    assert_eq!(
        GroupTelegram::parse(&read).unwrap().service,
        GroupService::Read
    );
    assert!(
        GroupTelegram::parse(&read)
            .unwrap()
            .indicated_value()
            .is_none()
    );
    let con = GroupTelegram::parse(&confirmation).unwrap();
    assert_eq!(
        con.primitive,
        LinkPrimitive::Confirmation { successful: true }
    );
    assert!(con.indicated_value().is_none());
    let ind = GroupTelegram::parse(&response).unwrap();
    assert_eq!(ind.source, 0x1101);
    assert_eq!(ind.destination.to_string(), "1/2/1");
    assert_eq!(
        decode_switch(ind.indicated_value().unwrap().as_bytes()),
        Ok(true)
    );
    let mut failed = confirmation;
    failed[2] |= 1;
    let con = GroupTelegram::parse(&failed).unwrap();
    assert_eq!(
        con.primitive,
        LinkPrimitive::Confirmation { successful: false }
    );
    assert!(con.indicated_value().is_none());
    let mut data = [0; 11];
    con.encode(&mut data).unwrap();
    assert_eq!(data, failed);
}

#[test]
fn extended_value_temperature_and_timestamp() {
    let frame = [
        0x29, 4, 4, 2, 0x12, 0x34, 0xbc, 0xe0, 0x11, 1, 0x0a, 1, 3, 0, 0x40, 7, 0xd0,
    ];
    let telegram = GroupTelegram::parse(&frame).unwrap();
    assert_eq!(telegram.relative_timestamp, Some(0x1234));
    assert_eq!(
        decode_temperature_celsius(telegram.indicated_value().unwrap().as_bytes()),
        Ok(20.0)
    );
    let mut out = [0; 17];
    telegram.encode(&mut out).unwrap();
    assert_eq!(out, frame);
}

#[test]
fn invalid_record_is_never_a_group_observation() {
    for cut in 0..SWITCH.len() {
        assert!(GroupTelegram::parse(&SWITCH[..cut]).is_err());
    }
    for (i, byte) in [
        (0, 0x2b),
        (1, 1),
        (2, 0xfc),
        (2, 0xac),
        (2, 0xbd),
        (3, 0x60),
        (3, 0xe4),
        (8, 0),
        (8, 2),
        (9, 1),
        (9, 0x40),
        (10, 0xc0),
        (10, 1),
    ] {
        let mut frame = SWITCH;
        frame[i] = byte;
        assert!(GroupTelegram::parse(&frame).is_err());
    }
    let mut frame = SWITCH;
    frame[6..8].fill(0);
    assert!(GroupTelegram::parse(&frame).is_err());
    let mut bad = SWITCH.to_vec();
    bad.push(1);
    assert!(GroupTelegram::parse(&bad).is_err());
    bad[8] = 2;
    assert!(GroupTelegram::parse(&bad).is_err());
    bad[10] = 0;
    assert!(GroupTelegram::parse(&bad).is_err());
    bad[10] = 0x40;
    bad[8] = 16;
    bad.resize(26, 0);
    assert!(GroupTelegram::parse(&bad).is_err());
}

#[test]
fn request_encoding_is_bounded_and_atomic() {
    let address = "4/3/2".parse().unwrap();
    let mut request = GroupTelegram::request(0, address, GroupService::Write(GroupData::Inline(1)));
    let mut target = [0xab; 25];
    assert_eq!(
        request.encode(&mut target[..10]),
        Err(TelegramError::OutputTooSmall)
    );
    assert_eq!(target, [0xab; 25]);
    let n = request.encode(&mut target).unwrap();
    assert_eq!(
        &target[..n],
        &[0x11, 0, 0xbc, 0xe0, 0, 0, 0x23, 2, 1, 0, 0x81]
    );
    for data in [
        GroupData::Inline(64),
        GroupData::Bytes(&[]),
        GroupData::Bytes(&[1; 254]),
    ] {
        request.service = GroupService::Response(data);
        let original = target;
        assert_eq!(
            request.encode(&mut target),
            Err(TelegramError::InvalidPayload)
        );
        assert_eq!(target, original);
    }
    request.service = GroupService::Write(GroupData::Bytes(&[1; 14]));
    let n = request.encode(&mut target).unwrap();
    assert_eq!(n, target.len());
    assert_eq!(GroupTelegram::parse(&target).unwrap(), request);
}

#[test]
fn malformed_metadata_and_oversized_records_have_bounded_static_errors() {
    for info in [
        &[4, 1, 0][..],
        &[4, 2, 0],
        &[0, 0],
        &[2, 8, 0, 0, 0, 0, 0, 0, 0, 0],
    ] {
        let mut frame = vec![0x29, info.len() as u8];
        frame.extend_from_slice(info);
        frame.extend_from_slice(&SWITCH[2..]);
        assert_eq!(
            GroupTelegram::parse(&frame),
            Err(TelegramError::UnsupportedProfile)
        );
    }
    let mut oversized = vec![0; 65536];
    oversized[..SWITCH.len()].copy_from_slice(&SWITCH);
    assert_eq!(GroupTelegram::parse(&oversized), Err(TelegramError::Length));
    assert_eq!(
        TelegramError::Length.to_string(),
        "Supply one complete cEMI frame with a valid length."
    );
}

#[test]
fn ordinary_extended_frames_match_calimero_at_each_size_boundary() {
    // Calimero CEMILDataEx with LOW priority; see 011 research for the generator.
    let values: Vec<u8> = (0..253).collect();
    for (n, control, length) in [(14, 0xbc, 15), (15, 0x3c, 16), (253, 0x3c, 254)] {
        let request = GroupTelegram::request(
            0x1101,
            "1/2/1".parse().unwrap(),
            GroupService::Write(GroupData::Bytes(&values[..n])),
        );
        let mut expected = vec![0x11, 0, control, 0xe0, 0x11, 1, 0x0a, 1, length, 0, 0x80];
        expected.extend_from_slice(&values[..n]);
        assert_eq!(GroupTelegram::parse(&expected).unwrap(), request);
        let mut out = [0; GroupTelegram::MAX_ENCODED_LEN];
        let size = request.encode(&mut out).unwrap();
        assert_eq!(&out[..size], &expected);
        if n > 14 {
            expected[2] |= 0x80;
            assert_eq!(GroupTelegram::parse(&expected), Err(TelegramError::Length));
            expected[2] &= 0x7f;
            expected[3] |= 4;
            assert_eq!(
                GroupTelegram::parse(&expected),
                Err(TelegramError::UnsupportedProfile)
            );
        }
    }
    let mut frame = SWITCH;
    frame[2] &= 0x7f;
    let telegram = GroupTelegram::parse(&frame).unwrap();
    let mut encoded = [0; 11];
    telegram.encode(&mut encoded).unwrap();
    assert_eq!(encoded, frame);

    let mut escaped = vec![0x29, 0, 0x3c, 0xe0, 0x11, 1, 0x0a, 1, 255, 0, 0x80];
    escaped.resize(11 + 254, 0);
    assert_eq!(GroupTelegram::parse(&escaped), Err(TelegramError::Length));
}

#[test]
fn bus_monitor_decodes_calimero_captures_and_status_flags() {
    use knx::telegram::{BusMonitorIndication, MonitorTimestamp};
    let short = [0x2b, 7, 3, 1, 0x22, 4, 2, 0xff, 1, 0xcc];
    let wide = [0x2b, 9, 3, 1, 0xaf, 6, 4, 0xf0, 0x12, 0x34, 0x56, 0xcc];
    for (bytes, timestamp) in [
        (short.as_slice(), MonitorTimestamp::Short(0xff01)),
        (wide.as_slice(), MonitorTimestamp::Extended(0xf0123456)),
    ] {
        let parsed = BusMonitorIndication::parse(bytes).unwrap();
        assert_eq!(parsed.timestamp, timestamp);
        assert_eq!(parsed.raw_frame(), &[0xcc]);
        assert!(knx::telegram::GroupTelegram::parse(bytes).is_err());
        let mut reordered = bytes.to_vec();
        reordered[2..bytes.len() - 1].rotate_left(3);
        assert_eq!(BusMonitorIndication::parse(&reordered), Ok(parsed));
        for end in 0..bytes.len() {
            assert!(BusMonitorIndication::parse(&bytes[..end]).is_err());
        }
    }
    for status in 0..=u8::MAX {
        let mut frame = short;
        frame[4] = status;
        let m = BusMonitorIndication::parse(&frame).unwrap();
        assert_eq!(m.sequence(), status % 8);
        assert_eq!(
            [
                m.lost(),
                m.overflow(),
                m.parity_error(),
                m.bit_error(),
                m.frame_error()
            ],
            [8, 16, 32, 64, 128].map(|bit| status & bit != 0)
        );
    }
}

#[test]
fn bus_monitor_rejects_ambiguous_metadata_and_checks_raw_bounds() {
    use knx::telegram::BusMonitorIndication;
    for info in [
        &[3, 1, 0, 3, 1, 0][..],
        &[4, 2, 0, 0, 6, 4, 0, 0, 0, 0],
        &[3, 1, 0],
        &[4, 2, 0, 0],
        &[3, 1, 0, 4, 1, 0],
        &[3, 1, 0, 4, 2, 0, 0, 99, 0],
        &[3, 1, 0, 4, 2, 0, 0, 3, 1, 1],
        &[3, 1, 0, 4, 2, 0, 0, 6, 4, 0, 0, 0, 0],
    ] {
        let mut frame = vec![0x2b, info.len() as u8];
        frame.extend_from_slice(info);
        frame.push(0xcc);
        assert!(BusMonitorIndication::parse(&frame).is_err());
    }
    let mut frame = vec![0x2b, 7, 3, 1, 0, 4, 2, 0, 0];
    let raw = std::array::from_fn::<_, 254, _>(|i| i as u8);
    frame.extend_from_slice(&raw);
    let capture = BusMonitorIndication::parse(&frame).unwrap();
    assert_eq!(capture.raw_frame().as_ptr(), frame[9..].as_ptr());
    assert_eq!(capture.raw_frame(), raw);
    assert!(!format!("{capture:?}").contains("0, 1, 2, 3"));
    frame[0] = 0x29;
    assert!(BusMonitorIndication::parse(&frame).is_err());
    frame[0] = 0x2b;
    frame.push(0);
    assert!(BusMonitorIndication::parse(&frame).is_err());
}

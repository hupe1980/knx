use knx::media::{Medium, Metadata};

#[test]
fn selected_radio_domain_and_tp_preserve_the_same_group_service() {
    let tp = [0x29, 0, 0xbc, 0xe0, 0x11, 2, 0x23, 2, 1, 0, 0x81];
    let rf = [
        0x29, 10, 2, 8, 2, 1, 2, 3, 4, 5, 6, 7, 0x10, 0xe0, 0x11, 2, 0x23, 2, 1, 0, 0x81,
    ];
    let domain = [1, 2, 3, 4, 5, 6];
    let radio = Medium::Radio { domain };
    let a = Medium::TwistedPair.indication(&tp).unwrap();
    let b = radio.indication(&rf).unwrap();
    assert_eq!(
        (a.source, a.destination, a.service),
        (b.source, b.destination, b.service)
    );
    assert_eq!((a.hop_count, b.hop_count), (6, 6));
    assert_eq!(
        b.medium,
        Metadata::Radio {
            domain,
            info: 2,
            frame_number: 7
        }
    );
    let mut extended = rf.to_vec();
    extended[20] = 0x80;
    extended[18] = 254;
    extended.extend_from_slice(&[0; 253]);
    assert!(radio.indication(&extended).is_ok());
    let mut tp_extended = vec![0x29, 0];
    tp_extended.extend_from_slice(&extended[12..]);
    assert!(Medium::TwistedPair.indication(&tp_extended).is_ok());
    extended[18] = 255;
    extended.push(0);
    assert!(radio.indication(&extended).is_err());
    for len in 0..rf.len() {
        assert!(radio.indication(&rf[..len]).is_err());
    }
    assert!(Medium::Radio { domain: [0; 6] }.indication(&rf).is_err());
    assert!(Medium::TwistedPair.indication(&rf).is_err());
    assert!(radio.indication(&tp).is_err());
    for (index, value) in [(0, 0x11), (11, 8), (12, 0), (13, 0x60), (18, 0), (20, 0xc1)] {
        let mut bad = rf;
        bad[index] = value;
        assert!(radio.indication(&bad).is_err());
    }
}

#[test]
fn pl110_requires_one_matching_domain_and_preserves_optional_time() {
    // Calimero core 68c99bf, CEMILDataEx.ind with PlMedium and Timestamp.
    let frame = [
        0x29, 8, 1, 2, 0, 0x34, 4, 2, 0x56, 0x78, 0x9c, 0xe0, 0x11, 2, 0x0a, 1, 1, 0, 0x81,
    ];
    let domain = std::num::NonZeroU8::new(0x34).unwrap();
    let medium = Medium::PowerLine { domain };
    let parsed = medium.indication(&frame).unwrap();
    assert_eq!(
        parsed.medium,
        Metadata::PowerLine {
            domain,
            timestamp: Some(0x5678)
        }
    );
    assert_eq!(parsed.source, 0x1102);
    assert_eq!(parsed.destination.to_string(), "1/2/1");
    assert_eq!(
        parsed.service,
        knx::telegram::GroupService::Write(knx::telegram::GroupData::Inline(1))
    );
    let mut reordered = frame;
    reordered[2..10].rotate_left(4);
    assert_eq!(medium.indication(&reordered), Ok(parsed));

    let mut untimed = vec![0x29, 4];
    untimed.extend_from_slice(&frame[2..6]);
    untimed.extend_from_slice(&frame[10..]);
    assert_eq!(
        medium.indication(&untimed).unwrap().medium,
        Metadata::PowerLine {
            domain,
            timestamp: None
        }
    );
    for i in 0..frame.len() {
        assert!(medium.indication(&frame[..i]).is_err());
    }
    for (i, b) in [
        (0, 0x11),
        (3, 1),
        (4, 1),
        (5, 0),
        (5, 0x35),
        (6, 1),
        (7, 3),
        (10, 0x1c),
        (10, 0x8c),
        (11, 0x60),
        (16, 2),
    ] {
        let mut changed = frame;
        changed[i] = b;
        assert!(medium.indication(&changed).is_err());
    }
    assert!(Medium::TwistedPair.indication(&frame).is_err());
    assert!(Medium::Radio { domain: [0; 6] }.indication(&frame).is_err());
    let mut no_domain = vec![0x29, 0];
    no_domain.extend_from_slice(&frame[10..]);
    assert!(medium.indication(&no_domain).is_err());
}

#[test]
fn group_filter_distinguishes_clear_set_and_outside_legacy_table() {
    use knx::{GroupAddress, media::GroupFilterTable};
    let mut bitmap = [0; 8192];
    // Raw 1, 7, 8 and the last group in each address range.
    bitmap[0] = 0x82;
    bitmap[1] = 0x01;
    bitmap[3583] = 0x80;
    bitmap[8191] = 0x80;
    let legacy = GroupFilterTable::from_bytes(&bitmap[..3584]).unwrap();
    let full = GroupFilterTable::from_bytes(&bitmap).unwrap();
    for (raw, expected) in [
        (1, true),
        (2, false),
        (7, true),
        (8, true),
        (9, false),
        (0x6fff, true),
        (0x7000, false),
        (0xffff, true),
    ] {
        let group = GroupAddress::try_from(raw).unwrap();
        assert_eq!(full.entry(group), Some(expected));
        assert_eq!(legacy.entry(group), (raw < 0x7000).then_some(expected));
    }
    for len in [0, 3583, 3585, 8191] {
        assert!(GroupFilterTable::from_bytes(&bitmap[..len]).is_none());
    }
}

#[test]
fn pl110_enforces_its_group_range_and_preserves_all_hop_counts() {
    let pl = Medium::PowerLine {
        domain: 1.try_into().unwrap(),
    };
    let mut frame = [
        0x29, 4, 1, 2, 0, 1, 0xbc, 0xe0, 0x11, 1, 0x7f, 0xff, 1, 0, 0x81,
    ];
    for hops in 0..=7 {
        frame[7] = 0x80 | hops << 4;
        let event = pl.indication(&frame).unwrap();
        assert_eq!(event.hop_count, hops);
        assert_eq!(u16::from(event.destination), 0x7fff);
    }
    frame[10..12].copy_from_slice(&0x8000u16.to_be_bytes());
    assert!(pl.indication(&frame).is_err());
    let mut tp = vec![0x29, 0];
    tp.extend_from_slice(&frame[6..]);
    assert_eq!(
        u16::from(Medium::TwistedPair.indication(&tp).unwrap().destination),
        0x8000
    );
}

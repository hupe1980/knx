use knx::{registers::*, text_time::DateTime};

#[test]
fn width_label_and_timestamp_have_independent_contracts() {
    let layout = RegisterLayout::from_dpt("281.1200").unwrap();
    let fields = [RegisterFields::default(); 4];
    let mut out = [99; 45];
    for (label, stamp, records) in [
        ("short", Some(DateTime::Fault), fields.as_slice()),
        ("ERL-METER01", None, fields.as_slice()),
        ("ERL-METER01", Some(DateTime::Fault), &fields[..3]),
        ("1234567890\0", Some(DateTime::Fault), fields.as_slice()),
    ] {
        assert!(layout.encode(label, stamp, records, &mut out).is_err());
        assert_eq!(out, [99; 45]);
    }
    assert!(
        layout
            .encode(
                "ERL-METER01",
                Some(DateTime::Fault),
                &fields,
                &mut out[..43]
            )
            .is_err()
    );
    assert_eq!(out, [99; 45]);
    let wire = layout
        .encode("abcdefghßz", Some(DateTime::Fault), &fields, &mut out)
        .unwrap();
    assert_eq!(wire.len(), 44);
    let parsed = layout.decode(wire).unwrap();
    assert_eq!(parsed.label, "abcdefghßz");
    assert_eq!(parsed.registers().collect::<Vec<_>>(), fields);
    assert_eq!(parsed.timestamp, Some(DateTime::Fault));
    assert_eq!(out[44], 99);
    for i in [8, 18, 19] {
        let mut bad = out;
        bad[i] = 0xff;
        assert!(layout.decode(&bad[..44]).is_err());
    }
    let mut early = out;
    early[8] = 0;
    assert!(layout.decode(&early[..44]).is_err());
    assert!(layout.decode(&out).is_err());
    assert!(layout.decode(&out[..43]).is_err());
}

#[test]
fn only_valid_count_and_selector_are_returned() {
    let layout = RegisterLayout::from_dpt("277.1200").unwrap();
    let mut b = [255; 36];
    b[..12].copy_from_slice(b"ERL-METER01\0");
    for (slot, flag) in b[12..].as_chunks_mut::<6>().0.iter_mut().zip(0..4) {
        slot[5] = flag;
    }
    let r = layout.decode(&b).unwrap();
    let fields: Vec<_> = r.registers().collect();
    assert_eq!(
        fields,
        [
            RegisterFields {
                count: Some(-1),
                selector: Some(255)
            },
            RegisterFields {
                count: Some(-1),
                selector: None
            },
            RegisterFields {
                count: None,
                selector: Some(255)
            },
            RegisterFields::default()
        ]
    );
    let mut out = [0; 36];
    layout.encode(r.label, None, &fields, &mut out).unwrap();
    assert_eq!(&out[18..24], &[255, 255, 255, 255, 0, 1]);
    assert_eq!(&out[30..], &[0, 0, 0, 0, 0, 3]);
}

#[test]
fn every_envelope_rejects_reserved_bits_and_inexact_lengths() {
    for (id, prefix, len) in [
        ("277.1200", 0, 36),
        ("278.1200", 0, 42),
        ("279.1200", 0, 48),
        ("280.1200", 0, 78),
        ("281.1200", 8, 44),
        ("282.1200", 8, 50),
        ("283.1200", 8, 56),
        ("284.1200", 8, 86),
    ] {
        let layout = RegisterLayout::from_dpt(id).unwrap();
        let mut wire = vec![0; len + 1];
        if prefix == 8 {
            wire[6] = 0x80; // Faulted source clock; register validity is independent.
        }
        wire[prefix..prefix + 12].copy_from_slice(b"ERL-METER01\0");
        assert!(layout.decode(&wire).is_err(), "{id} trailing byte");
        assert!(
            layout.decode(&wire[..len - 1]).is_err(),
            "{id} missing byte"
        );
        for slot_flags in (prefix + 17..len).step_by(6) {
            for flags in 0..=255 {
                wire[slot_flags] = flags;
                assert_eq!(
                    layout.decode(&wire[..len]).is_ok(),
                    flags & 0xfc == 0,
                    "{id} at {slot_flags}"
                );
            }
            wire[slot_flags] = 0;
        }
    }
}

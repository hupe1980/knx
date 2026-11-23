use knx::parameters::{Binding, PARAMETERS, ParameterError, ParameterType, Validation};
use std::collections::BTreeSet;

fn get(id: u16) -> ParameterType {
    ParameterType::from_id(id).unwrap()
}

#[test]
fn catalog_matches_every_parameter_and_separates_group_directions() {
    let mut names = BTreeSet::new();
    let mut numbers = BTreeSet::new();
    for line in include_str!("../site/static/standards.csv").lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        if !matches!(fields[1], "parameter" | "unassigned-parameter") {
            continue;
        }
        let p = ParameterType::from_name(fields[3]).unwrap();
        assert!(names.insert(p.name()));
        if fields[2].is_empty() {
            assert_eq!(p.id(), None)
        } else {
            let id = u16::from_str_radix(fields[2].strip_suffix('h').unwrap(), 16).unwrap();
            assert_eq!(Some(id), p.id());
            assert_eq!(get(id), p);
            assert!(numbers.insert(id));
        }
        match p.binding() {
            Binding::Datapoint(id) | Binding::Unresolved(id) => assert_eq!(fields[10], id),
            Binding::None => assert!(matches!(fields[10], "" | "(none)")),
        }
        let validation = match p.validation() {
            Validation::Value => "value",
            Validation::Layout => "layout",
            Validation::Unspecified => "unspecified",
        };
        assert_eq!(fields[11], validation);
        assert_eq!(
            &fields[6..10],
            &["not-applicable", "not-applicable", "", ""]
        );
    }
    assert_eq!(names.len(), 36);
    assert_eq!(numbers.len(), 32);
    assert_eq!(names.len(), PARAMETERS.len());
    assert_eq!(ParameterType::from_id(0), None);
    assert_eq!(ParameterType::from_name("COV_Power"), None);
}

#[test]
fn chapter_11_acceptance_values_preserve_the_supplied_encoding() {
    let vectors: &[(u16, &[u8])] = &[
        (0x0101, &[1]),
        (0x0102, &[0]),
        (0x0103, &[1]),
        (0x0104, &[1]),
        (0x0105, &[1]),
        (0x0106, &[1]),
        (0x0107, &[1]),
        (0x0108, &[1]),
        (0x1101, &[7]),
        (0x1301, &[0, 0, 0, 0, 0, 0, 128, 0]),
        (0x1401, &[15]),
        (0x1402, &[25]),
        (0x1403, &[6]),
        (0x1404, &[4]),
        (0x1405, &[2]),
        (0xe401, &[0xfb]),
        (0x1701, &[3]),
        (0x1702, &[2]),
        (0x1703, &[3]),
        (0x1704, &[3]),
        (0x1705, &[3]),
        (0x0501, &[255]),
        (0x0502, &[255]),
        (0x0503, &[0]),
        (0x0504, &[254]),
        (0x0505, &[253]),
        (0x0506, &[128]),
        (0x0701, &[255, 255]),
        (0x0702, &[0x12, 0x34]),
        (0x0703, &[0, 5]),
        (0x0901, &[8, 1]),
        (0x0902, &[0x80, 1]),
    ];
    assert_eq!(
        vectors.iter().map(|x| x.0).collect::<BTreeSet<_>>().len(),
        32
    );
    for &(id, wire) in vectors {
        let param = get(id);
        let value = param.validate(wire).unwrap();
        assert_eq!(value.parameter(), param);
        assert_eq!(value.as_bytes(), wire);
        assert!(param.validate(&wire[..wire.len() - 1]).is_err());
    }
}

#[test]
fn parameter_ranges_are_narrower_than_some_dpt_bindings() {
    for value in 0..=u8::MAX {
        for (id, valid) in [
            (0x0101, value <= 1),
            (0x1101, value <= 7),
            (0x1401, matches!(value, 5 | 8 | 9 | 10 | 13 | 15)),
            (0x1402, value <= 25),
            (0x1403, matches!(value, 0 | 6 | 8 | 10)),
            (0x1702, value <= 2),
            (0x1705, value <= 3),
            (0xe401, matches!(value & 7, 1..=3)),
        ] {
            assert_eq!(get(id).validate(&[value]).is_ok(), valid);
        }
    }
    assert!(get(0x0901).validate(&[0x7f, 0xff]).is_err());
    assert!(get(0x0901).validate(&[0x80, 1]).is_err());
    assert_eq!(get(0xe401).validation(), Validation::Layout);
    assert_eq!(get(0x1705).validation(), Validation::Layout);
}

#[test]
fn unnumbered_parameters_never_acquire_a_wire_id() {
    let mask = ParameterType::from_name("PART_Input_Connected").unwrap();
    assert_eq!(mask.id(), None);
    assert_eq!(mask.validate(&[10]).unwrap().as_bytes(), &[10]);
    assert!(mask.validate(&[16]).is_err());
    for name in [
        "PART_PB_Action_HVAC_Extended",
        "PART_COV_Power",
        "PART_COV_Energy",
    ] {
        let p = ParameterType::from_name(name).unwrap();
        assert_eq!(p.id(), None);
        assert_eq!(p.validation(), Validation::Unspecified);
        assert_eq!(p.validate(&[0; 4]), Err(ParameterError::UnspecifiedFormat));
    }
}

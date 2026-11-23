use knx::{CodecError, Energy32, Energy64};

#[test]
fn four_byte_registers_preserve_native_units_and_signed_endpoints() {
    for (id, unit, register) in [
        ("13.010", "Wh", Energy32::ActiveWh),
        ("13.011", "VAh", Energy32::ApparentVah),
        ("13.012", "VARh", Energy32::ReactiveVarh),
        ("13.013", "kWh", Energy32::ActiveKwh),
        ("13.014", "kVAh", Energy32::ApparentKvah),
        ("13.015", "kVARh", Energy32::ReactiveKvarh),
        ("13.016", "MWh", Energy32::ActiveMwh),
    ] {
        assert_eq!(Energy32::from_dpt(id), Some(register));
        assert_eq!(Energy64::from_dpt(id), None);
        assert_eq!(register.unit(), unit);
        for (count, bytes) in [
            (i32::MIN, [128, 0, 0, 0]),
            (i32::MAX, [127, 255, 255, 255]),
            (0, [0; 4]),
            (-2, [255, 255, 255, 254]),
            (0x01234567, [1, 35, 69, 103]),
        ] {
            assert_eq!(register.encode(count), bytes);
            assert_eq!(register.decode(&bytes), Ok(count));
        }
        for len in [0, 3, 5, 8] {
            assert_eq!(
                register.decode(&[0; 8][..len]),
                Err(CodecError::PayloadLength {
                    expected: 4,
                    actual: len
                })
            );
        }
    }
}

#[test]
fn eight_byte_registers_preserve_precision_and_select_the_matching_companion() {
    for (id, unit, register, companion) in [
        ("29.010", "Wh", Energy64::ActiveWh, Energy32::ActiveWh),
        (
            "29.011",
            "VAh",
            Energy64::ApparentVah,
            Energy32::ApparentVah,
        ),
        (
            "29.012",
            "VARh",
            Energy64::ReactiveVarh,
            Energy32::ReactiveVarh,
        ),
    ] {
        assert_eq!(Energy64::from_dpt(id), Some(register));
        assert_eq!(Energy32::from_dpt(id), None);
        assert_eq!(register.companion(), companion);
        assert_eq!(register.unit(), unit);
        for (count, bytes) in [
            (i64::MIN, [128, 0, 0, 0, 0, 0, 0, 0]),
            (i64::MAX, [127, 255, 255, 255, 255, 255, 255, 255]),
            (0, [0; 8]),
            (-2, [255, 255, 255, 255, 255, 255, 255, 254]),
            (9_007_199_254_740_993, [0, 32, 0, 0, 0, 0, 0, 1]),
            (0x0123456789abcdef, [1, 35, 69, 103, 137, 171, 205, 239]),
        ] {
            assert_eq!(register.encode(count), bytes);
            assert_eq!(register.decode(&bytes), Ok(count));
        }
        for len in [0, 4, 7, 9] {
            assert_eq!(
                register.decode(&[0; 9][..len]),
                Err(CodecError::PayloadLength {
                    expected: 8,
                    actual: len
                })
            );
        }
    }
    for id in ["13", "29", "13.001", "13.017", "29.013", "13.10"] {
        assert_eq!(Energy32::from_dpt(id), None);
        assert_eq!(Energy64::from_dpt(id), None);
    }
}

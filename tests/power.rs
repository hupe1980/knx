use knx::{CodecError, decode_power_watts};

#[test]
fn decodes_signed_watts_in_network_order() {
    for (payload, watts) in [
        ([0x46, 0xbb, 0x80, 0], 24_000.0_f32),
        ([0xc6, 0xbb, 0x80, 0], -24_000.0),
        ([0, 0, 0, 0], 0.0),
        ([0x80, 0, 0, 0], -0.0),
        ([0, 0, 0, 1], f32::from_bits(1)),
        ([0x7f, 0x7f, 0xff, 0xff], f32::MAX),
    ] {
        assert_eq!(
            decode_power_watts(&payload).unwrap().to_bits(),
            watts.to_bits()
        );
    }
}

#[test]
fn distinguishes_invalid_width_from_nonfinite_power() {
    for len in [0, 1, 2, 3, 5, 8] {
        assert_eq!(
            decode_power_watts(&[0; 8][..len]),
            Err(CodecError::PayloadLength {
                expected: 4,
                actual: len
            })
        );
    }
    for bits in [0x7f800000u32, 0xff800000, 0x7fc00001, 0xff800001] {
        assert_eq!(
            decode_power_watts(&bits.to_be_bytes()),
            Err(CodecError::NonFinite)
        );
    }
}

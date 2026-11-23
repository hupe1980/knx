use knx::text_time::*;

#[test]
fn character_domains_and_fixed_padding() {
    for byte in 0..=255u8 {
        for charset in [Charset::Latin1, Charset::Ascii] {
            let decoded = charset.decode_char(&[byte]);
            if charset == Charset::Ascii && byte > 127 {
                assert!(decoded.is_err());
            } else {
                assert_eq!(decoded, Ok(char::from(byte)));
                assert_eq!(charset.encode_char(char::from(byte)), Ok([byte]));
            }
        }
    }
    assert_eq!(Charset::Ascii.encode_fixed(""), Ok([0; 14]));
    assert_eq!(Charset::Latin1.encode_fixed(&"ÿ".repeat(14)), Ok([255; 14]));
    assert!(Charset::Latin1.encode_fixed(&"ÿ".repeat(15)).is_err());
    assert!(Charset::Ascii.decode_fixed(&[128; 14]).is_err());
    assert!(Charset::Ascii.decode_fixed(b"KNX\0is bad\0\0\0\0").is_err());
    assert!(Charset::Latin1.encode_fixed("foo\0bar").is_err());
    assert!(Charset::Latin1.encode_fixed("₂").is_err());
    assert!(Charset::Ascii.decode_char(&[]).is_err());
    assert!(Charset::Ascii.decode_fixed(&[0; 13]).is_err());
}

#[test]
fn variable_strings_have_exactly_one_terminator_and_atomic_writes() {
    for malformed in [&[][..], b"abc", b"a\0b\0", b"\0\0"] {
        assert!(decode_latin1(malformed).is_err());
        assert!(decode_utf8(malformed).is_err());
    }
    for malformed in [
        &[0xc0, 0x80, 0][..],
        &[0xf4, 0x90, 0x80, 0x80, 0],
        &[0xe2, 0x82, 0],
    ] {
        assert!(decode_utf8(malformed).is_err());
    }
    let mut out = [0xff; 5];
    assert!(encode_utf8("ab\0c", &mut out).is_err());
    assert!(encode_utf8("🙂x", &mut out).is_err());
    assert!(encode_latin1("word🙂", &mut out).is_err());
    assert!(encode_latin1("abcde", &mut out).is_err());
    assert_eq!(out, [0xff; 5]);
    assert_eq!(
        encode_utf8("🙂", &mut out),
        Ok([0xf0, 0x9f, 0x99, 0x82, 0].as_slice())
    );
    assert_eq!(decode_utf8(&out), Ok("🙂"));
    assert_eq!(encode_latin1("", &mut out), Ok([0].as_slice()));
    assert_eq!(decode_latin1(&[0]), Ok([].as_slice()));
}

#[test]
fn dates_check_century_pivot_and_impossible_fields() {
    for (wire, year) in [(89, 2089), (90, 1990), (99, 1999), (0, 2000)] {
        let date = Date {
            year,
            month: 12,
            day: 31,
        };
        assert_eq!(Date::decode(&[31, 12, wire]), Ok(date));
        assert_eq!(date.encode(), Ok([31, 12, wire]));
    }
    for bad in [[1, 12, 100], [29, 2, 99], [31, 4, 0], [0, 1, 0], [1, 13, 0]] {
        assert!(Date::decode(&bad).is_err());
    }
    assert!(Date::decode(&[1, 1]).is_err());
    assert!(
        Date {
            year: 2090,
            month: 1,
            day: 1
        }
        .encode()
        .is_err()
    );
    for bad in [[24, 0, 0], [0, 60, 0], [0, 0, 60]] {
        assert!(TimeOfDay::decode(&bad).is_err());
    }
    assert!(TimeOfDay::decode(&[1, 1]).is_err());
}

#[test]
fn date_time_retains_absence_wildcard_clock_quality_and_end_of_day() {
    let wire = [100, 2, 29, 24, 0, 0, 0x41, 0xc0];
    let fields = CalendarFields {
        year: Some(2000),
        month_day: Some((2, 29)),
        weekday: Some(0),
        time: Some(Time {
            hour: 24,
            minute: 0,
            second: 0,
        }),
        working_day: Some(true),
        summer_time: true,
        externally_synchronized: true,
        reliable_source: true,
    };
    assert_eq!(decode_date_time(&wire), Ok(DateTime::Value(fields)));
    assert_eq!(encode_date_time(DateTime::Value(fields)), Ok(wire));
    let absent = DateTime::Value(CalendarFields::default());
    assert_eq!(
        decode_date_time(&[255, 255, 255, 255, 255, 255, 0x3e, 0]),
        Ok(absent)
    );
    assert_eq!(encode_date_time(absent), Ok([0, 0, 0, 0, 0, 0, 0x3e, 0]));
    for i in [0, 200] {
        let mut invalid = wire;
        invalid[0] = i;
        assert!(decode_date_time(&invalid).is_err()); // 1900 and 2100 were common years.
    }
    for (index, invalid) in [(4, 1), (5, 1), (1, 0), (2, 0), (7, 1)] {
        let mut bad = wire;
        bad[index] = invalid;
        assert!(decode_date_time(&bad).is_err());
    }
    assert_eq!(
        decode_date_time(&[0, 0, 0, 0, 0, 0, 0x80, 0]),
        Ok(DateTime::Fault)
    );
    assert_eq!(
        encode_date_time(DateTime::Fault),
        Ok([0, 0, 0, 0, 0, 0, 0x80, 0])
    );
    assert!(decode_date_time(&[]).is_err());
}

#[test]
fn embedded_clock_layouts_apply_their_own_reserved_bits() {
    use knx::{
        DatedTariffEnergy,
        domain::{MeterRole, PhaseQuantity, dali::ConverterTestInfo},
    };
    for quality in 0..=255 {
        let mut payload = [0u8; 20];
        payload[6] = 0x3e;
        payload[7] = quality;
        assert_eq!(decode_date_time(&payload[..8]).is_ok(), quality & 0x3f == 0);
        assert_eq!(
            MeterRole::decode_dated(&payload[..9]).is_ok(),
            quality & 0x3f == 0
        );
        assert_eq!(
            PhaseQuantity::Current.decode_dated(&payload).is_ok(),
            quality & 0x3f == 0
        );
        assert_eq!(
            ConverterTestInfo::decode(&payload[..12]).is_ok(),
            quality & 0x7f == 0
        );
        assert_eq!(
            DatedTariffEnergy::decode(&payload[..14]).is_ok(),
            quality & 0x7f == 0
        );
    }
}

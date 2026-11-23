//! KNX character, string and local calendar application values.
//!
//! Calendar decoders mask the unused high bits of time and date components. The
//! clock-quality octet rejects reserved bits. Encoders clear unused fields.
//!
//! ```
//! use knx::text_time::{DateTime, decode_date_time};
//! let DateTime::Value(fields) = decode_date_time(&[0, 2, 29, 0, 0, 0, 0x36, 0])? else {
//!     panic!("source reported a clock fault")
//! };
//! assert_eq!(fields.year, None);
//! assert_eq!(fields.month_day, Some((2, 29)));
//! # Ok::<(), knx::CodecError>(())
//! ```

use crate::{CodecError, codecs::bytes};

/// Commissioned character encoding for DPT 4 and 16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    Ascii,
    Latin1,
}

impl Charset {
    /// Decode a DPT 4.001 or 4.002 character, including NUL. The host decides
    /// whether to display or act on control characters.
    pub fn decode_char(self, payload: &[u8]) -> Result<char, CodecError> {
        let [byte] = bytes(payload)?;
        if self == Self::Ascii && !byte.is_ascii() {
            return Err(CodecError::OutsideRange);
        }
        Ok(char::from(byte))
    }

    /// Encode one character without replacement or transliteration.
    pub fn encode_char(self, value: char) -> Result<[u8; 1], CodecError> {
        let max = if self == Self::Ascii { 127 } else { 255 };
        if u32::from(value) > max {
            return Err(CodecError::OutsideRange);
        }
        Ok([value as u8])
    }

    /// Return the unpadded DPT 16.000 or 16.001 characters. Each byte maps to its Unicode scalar.
    pub fn decode_fixed(self, payload: &[u8]) -> Result<&[u8], CodecError> {
        let _: &[u8; 14] = payload.try_into().map_err(|_| CodecError::PayloadLength {
            expected: 14,
            actual: payload.len(),
        })?;
        let end = payload.iter().position(|&b| b == 0).unwrap_or(14);
        if payload[end..].iter().any(|&b| b != 0)
            || (self == Self::Ascii && !payload[..end].is_ascii())
        {
            return Err(CodecError::InvalidData);
        }
        Ok(&payload[..end])
    }

    /// Encode up to 14 characters with zero padding for unused bytes.
    pub fn encode_fixed(self, text: &str) -> Result<[u8; 14], CodecError> {
        let mut out = [0; 14];
        for (index, ch) in text.chars().enumerate() {
            if ch == '\0' || index >= out.len() {
                return Err(CodecError::OutsideRange);
            }
            out[index] = self.encode_char(ch)?[0];
        }
        Ok(out)
    }
}

/// Decode one complete NUL-terminated DPT 24.001 string to Latin-1 character bytes.
pub fn decode_latin1(payload: &[u8]) -> Result<&[u8], CodecError> {
    match payload.split_last() {
        Some((&0, text)) if !text.contains(&0) => Ok(text),
        _ => Err(CodecError::InvalidData),
    }
}

/// Decode one complete NUL-terminated DPT 28.001 string, checking UTF-8.
pub fn decode_utf8(payload: &[u8]) -> Result<&str, CodecError> {
    std::str::from_utf8(decode_latin1(payload)?).map_err(|_| CodecError::InvalidData)
}

fn output(out: &mut [u8], len: usize) -> Result<&mut [u8], CodecError> {
    let expected = len + 1;
    let actual = out.len();
    out.get_mut(..expected)
        .ok_or(CodecError::PayloadLength { expected, actual })
}

/// Encode a DPT 24.001 string into caller storage. Return the written bytes including NUL.
pub fn encode_latin1<'a>(text: &str, out: &'a mut [u8]) -> Result<&'a [u8], CodecError> {
    let mut count = 0;
    for ch in text.chars() {
        if ch == '\0' || u32::from(ch) > 255 {
            return Err(CodecError::OutsideRange);
        }
        count += 1;
    }
    let dst = output(out, count)?;
    for (byte, ch) in dst.iter_mut().zip(text.chars()) {
        *byte = ch as u8;
    }
    dst[count] = 0;
    Ok(dst)
}

/// Encode a DPT 28.001 string into caller storage. Return the written bytes including NUL.
pub fn encode_utf8<'a>(text: &str, out: &'a mut [u8]) -> Result<&'a [u8], CodecError> {
    if text.as_bytes().contains(&0) {
        return Err(CodecError::OutsideRange);
    }
    let dst = output(out, text.len())?;
    dst[..text.len()].copy_from_slice(text.as_bytes());
    dst[text.len()] = 0;
    Ok(dst)
}

/// Local wall-clock fields. Only DPT 19.001 accepts end of day (24:00:00).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Time {
    fn validate(self, end_of_day: bool) -> Result<(), CodecError> {
        if (self.hour < 24 && self.minute < 60 && self.second < 60)
            || (end_of_day && self.hour == 24 && self.minute == 0 && self.second == 0)
        {
            Ok(())
        } else {
            Err(CodecError::OutsideRange)
        }
    }
}

/// DPT 10.001 local time. Weekday 0 is unspecified, 1 is Monday and 7 is Sunday.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub weekday: u8,
    pub time: Time,
}

impl TimeOfDay {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [hour, minute, second] = bytes(payload)?;
        let time = Time {
            hour: hour & 31,
            minute: minute & 63,
            second: second & 63,
        };
        time.validate(false)?;
        Ok(Self {
            weekday: hour >> 5,
            time,
        })
    }

    pub fn encode(self) -> Result<[u8; 3], CodecError> {
        self.time.validate(false)?;
        if self.weekday > 7 {
            return Err(CodecError::OutsideRange);
        }
        Ok([
            self.weekday << 5 | self.time.hour,
            self.time.minute,
            self.time.second,
        ])
    }
}

fn validate_day(year: Option<u16>, month: u8, day: u8) -> Result<(), CodecError> {
    let leap = year
        .is_none_or(|y| y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400)));
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28 + u8::from(leap),
        _ => 0,
    };
    if (1..=last).contains(&day) {
        Ok(())
    } else {
        Err(CodecError::OutsideRange)
    }
}

/// DPT 11.001 Gregorian date in 1990–2089.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [day, month, year] = bytes(payload)?;
        let year = year & 127;
        if year > 99 {
            return Err(CodecError::OutsideRange);
        }
        let year = if year >= 90 { 1900 } else { 2000 } + u16::from(year);
        let month = month & 15;
        let day = day & 31;
        validate_day(Some(year), month, day)?;
        Ok(Self { year, month, day })
    }

    pub fn encode(self) -> Result<[u8; 3], CodecError> {
        if !(1990..=2089).contains(&self.year) {
            return Err(CodecError::OutsideRange);
        }
        validate_day(Some(self.year), self.month, self.day)?;
        Ok([self.day, self.month, (self.year % 100) as u8])
    }
}

/// Present DPT 19.001 fields. Times are already adjusted for the summer-time flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CalendarFields {
    pub year: Option<u16>,
    /// (Month, day). February 29 is valid if the year is unspecified.
    pub month_day: Option<(u8, u8)>,
    /// None is unsupported, Some(0) is any weekday, Some(1..=7) is Monday through Sunday.
    pub weekday: Option<u8>,
    pub time: Option<Time>,
    pub working_day: Option<bool>,
    pub summer_time: bool,
    pub externally_synchronized: bool,
    pub reliable_source: bool,
}

/// A fault renders the entire DPT 19.001 reading unusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTime {
    Fault,
    Value(CalendarFields),
}

/// Decode local calendar fields independently, ignoring values explicitly marked absent.
pub fn decode_date_time(payload: &[u8]) -> Result<DateTime, CodecError> {
    let [year, month, day, hour, minute, second, flags, quality] = bytes(payload)?;
    if quality & 0x3f != 0 {
        return Err(CodecError::InvalidData);
    }
    if flags & 0x80 != 0 {
        return Ok(DateTime::Fault);
    }
    let year = (flags & 0x10 == 0).then_some(1900 + u16::from(year));
    let month_day = (flags & 8 == 0).then_some((month & 15, day & 31));
    if let Some((month, day)) = month_day {
        validate_day(year, month, day)?;
    }
    let time = (flags & 2 == 0).then_some(Time {
        hour: hour & 31,
        minute: minute & 63,
        second: second & 63,
    });
    if let Some(time) = time {
        time.validate(true)?;
    }
    Ok(DateTime::Value(CalendarFields {
        year,
        month_day,
        time,
        weekday: (flags & 4 == 0).then_some(hour >> 5),
        working_day: (flags & 32 == 0).then_some(flags & 64 != 0),
        summer_time: flags & 1 != 0,
        externally_synchronized: quality & 0x80 != 0,
        reliable_source: quality & 0x40 != 0,
    }))
}

/// Encode DPT 19.001, zeroing reserved and absent fields. Encode faults canonically.
pub fn encode_date_time(value: DateTime) -> Result<[u8; 8], CodecError> {
    let DateTime::Value(fields) = value else {
        return Ok([0, 0, 0, 0, 0, 0, 0x80, 0]);
    };
    let mut raw = [0; 8];
    if let Some(year) = fields.year {
        if !(1900..=2155).contains(&year) {
            return Err(CodecError::OutsideRange);
        }
        raw[0] = (year - 1900) as u8;
    } else {
        raw[6] |= 0x10;
    }
    if let Some((month, day)) = fields.month_day {
        validate_day(fields.year, month, day)?;
        raw[1] = month;
        raw[2] = day;
    } else {
        raw[6] |= 8;
    }
    if let Some(weekday) = fields.weekday {
        if weekday > 7 {
            return Err(CodecError::OutsideRange);
        }
        raw[3] = weekday << 5;
    } else {
        raw[6] |= 4;
    }
    if let Some(time) = fields.time {
        time.validate(true)?;
        raw[3] |= time.hour;
        raw[4] = time.minute;
        raw[5] = time.second;
    } else {
        raw[6] |= 2;
    }
    raw[6] |= match fields.working_day {
        Some(true) => 64,
        Some(false) => 0,
        None => 32,
    };
    raw[6] |= u8::from(fields.summer_time);
    raw[7] = u8::from(fields.externally_synchronized) << 7 | u8::from(fields.reliable_source) << 6;
    Ok(raw)
}

/// The DPT 247.600 and 269.1200 timestamp layouts retain CLQ with seven reserved bits.
pub(crate) fn decode_clq_date_time(payload: &[u8]) -> Result<DateTime, CodecError> {
    let raw: &[u8; 8] = payload.try_into().map_err(|_| CodecError::PayloadLength {
        expected: 8,
        actual: payload.len(),
    })?;
    if raw[7] & 0x7f != 0 {
        return Err(CodecError::InvalidData);
    }
    decode_date_time(raw)
}

pub(crate) fn encode_clq_date_time(value: DateTime) -> Result<[u8; 8], CodecError> {
    let raw = encode_date_time(value)?;
    if raw[7] & 0x40 != 0 {
        return Err(CodecError::OutsideRange);
    }
    Ok(raw)
}

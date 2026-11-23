use knx::{
    BlindControl, CodecError, Dimming, Energy32, Energy64, PriorityControl, PriorityDpt,
    SceneControl, SceneNumber, StatusMode, StatusWithMode, StepSize, Tariff, TimeIntervalDpt,
    decode_angle_degrees, decode_blind_control, decode_decimal_factor, decode_dimming,
    decode_percent_i8, decode_percent_u8, decode_power_watts, decode_pulse_count,
    decode_scaling_percent, decode_scene_control, decode_scene_number, decode_signed_pulse_count,
    decode_switch, decode_temperature_celsius, encode_angle_degrees, encode_blind_control,
    encode_decimal_factor, encode_dimming, encode_percent_i8, encode_percent_u8,
    encode_power_watts, encode_pulse_count, encode_scaling_percent, encode_scene_control,
    encode_scene_number, encode_signed_pulse_count, encode_switch, encode_temperature_celsius,
};

pub const DPT_HELP: &str = "Commissioned subtype: 1.001, 2.001–2.012, 3.007, 3.008, 5.001, 5.003–5.006, 5.010, 6.001, 6.010, 6.020, 7.002–7.007, 8.002–8.007, 9.001, 13.010–13.016, 14.056, 17.001, 18.001 or 29.010–29.012.";

pub enum Dpt {
    Interval(TimeIntervalDpt),
    Priority(PriorityDpt),
    Dimming,
    Blinds,
    SceneNumber,
    SceneControl,
    Switch,
    Scaling,
    Angle,
    DecimalFactor,
    Tariff,
    PulseCount,
    SignedPercent,
    SignedPulses,
    Status,
    WholePercent,
    Temperature,
    Energy32(Energy32),
    Energy64(Energy64),
    Power,
}

impl Dpt {
    pub fn parse(id: &str) -> Result<Self, String> {
        match id {
            "3.007" => Ok(Self::Dimming),
            "3.008" => Ok(Self::Blinds),
            "17.001" => Ok(Self::SceneNumber),
            "18.001" => Ok(Self::SceneControl),
            "1.001" => Ok(Self::Switch),
            "5.001" => Ok(Self::Scaling),
            "5.003" => Ok(Self::Angle),
            "5.005" => Ok(Self::DecimalFactor),
            "5.006" => Ok(Self::Tariff),
            "5.010" => Ok(Self::PulseCount),
            "5.004" => Ok(Self::WholePercent),
            "6.001" => Ok(Self::SignedPercent),
            "6.010" => Ok(Self::SignedPulses),
            "6.020" => Ok(Self::Status),
            "9.001" => Ok(Self::Temperature),
            "14.056" => Ok(Self::Power),
            _ => Energy32::from_dpt(id)
                .map(Self::Energy32)
                .or_else(|| TimeIntervalDpt::from_dpt(id).map(Self::Interval))
                .or_else(|| PriorityDpt::from_dpt(id).map(Self::Priority))
                .or_else(|| Energy64::from_dpt(id).map(Self::Energy64))
                .ok_or_else(|| format!("Select a supported DPT. {DPT_HELP}")),
        }
    }

    pub fn decode(&self, hex: &[String]) -> Result<String, String> {
        let mut buffer = [0; 8];
        let mut len = 0;
        let invalid_hex = "Supply complete hexadecimal byte pairs, for example '07 D0'.";
        for group in hex.iter().flat_map(|arg| arg.split_ascii_whitespace()) {
            if !group.bytes().all(|b| b.is_ascii_hexdigit()) || !group.len().is_multiple_of(2) {
                return Err(invalid_hex.into());
            }
            for start in (0..group.len()).step_by(2) {
                let slot = buffer
                    .get_mut(len)
                    .ok_or("Supply at most eight application bytes.")?;
                *slot =
                    u8::from_str_radix(&group[start..start + 2], 16).map_err(|_| invalid_hex)?;
                len += 1;
            }
        }
        let payload = &buffer[..len];
        match self {
            Self::Interval(dpt) => dpt.decode_ms(payload).map(|ms| format!("{ms} ms")),
            Self::Priority(dpt) => dpt.decode(payload).map(|command| match command {
                PriorityControl::Release => "release".into(),
                PriorityControl::Set(false) => dpt.labels().0.into(),
                PriorityControl::Set(true) => dpt.labels().1.into(),
            }),
            Self::Dimming => decode_dimming(payload).map(|cmd| match cmd {
                Dimming::Stop => "stop".into(),
                Dimming::Increase(step) => format!("increase 1/{}", step.denominator()),
                Dimming::Decrease(step) => format!("decrease 1/{}", step.denominator()),
            }),
            Self::Blinds => decode_blind_control(payload).map(|cmd| match cmd {
                BlindControl::Stop => "stop".into(),
                BlindControl::Up(step) => format!("up 1/{}", step.denominator()),
                BlindControl::Down(step) => format!("down 1/{}", step.denominator()),
            }),
            Self::SceneNumber => decode_scene_number(payload).map(|s| s.number().to_string()),
            Self::SceneControl => decode_scene_control(payload).map(|cmd| match cmd {
                SceneControl::Activate(s) => format!("activate {}", s.number()),
                SceneControl::Learn(s) => format!("learn {}", s.number()),
            }),
            Self::Switch => {
                decode_switch(payload).map(|on| if on { "on".into() } else { "off".into() })
            }
            Self::Angle => decode_angle_degrees(payload).map(|v| format!("{v} °")),
            Self::DecimalFactor => decode_decimal_factor(payload).map(|v| format!("{v} ratio")),
            Self::PulseCount => decode_pulse_count(payload).map(|v| format_pulses(i16::from(v))),
            Self::Tariff => Tariff::decode(payload).map(|v| match v.number() {
                Some(n) => n.to_string(),
                None => "unavailable".into(),
            }),
            Self::SignedPercent => decode_percent_i8(payload).map(|v| format!("{v} %")),
            Self::SignedPulses => {
                decode_signed_pulse_count(payload).map(|v| format_pulses(i16::from(v)))
            }
            Self::Status => StatusWithMode::decode(payload).map(format_status),
            Self::WholePercent => decode_percent_u8(payload).map(|v| format!("{v} %")),
            Self::Scaling => decode_scaling_percent(payload).map(|v| format!("{v} %")),
            Self::Temperature => decode_temperature_celsius(payload).map(|v| format!("{v} °C")),
            Self::Energy32(dpt) => dpt.decode(payload).map(|v| format!("{v} {}", dpt.unit())),
            Self::Energy64(dpt) => dpt.decode(payload).map(|v| format!("{v} {}", dpt.unit())),
            Self::Power => decode_power_watts(payload).map(|v| format!("{v} W")),
        }
        .map_err(|err| match (self, err) {
            (Self::Status, CodecError::OutsideRange) => {
                "Check the three low mode bits. Exactly one must be set.".into()
            }
            (_, err) => err.to_string(),
        })
    }
    pub fn encode(&self, args: &[String]) -> Result<String, String> {
        let value = || match args {
            [value] => Ok(value.as_str()),
            _ => Err("Supply one value without a unit suffix."),
        };
        let decimal = "Supply a finite decimal number without a unit suffix.";
        match self {
            Self::Interval(dpt) => {
                let [value, unit] = args else {
                    return Err("Supply an integer interval and unit, for example 20 ms.".into());
                };
                let multiplier = match unit.as_str() {
                    "ms" => 1,
                    "s" => 1000,
                    "min" => 60_000,
                    "h" => 3_600_000,
                    _ => return Err("Use ms, s, min or h for a time interval.".into()),
                };
                let n: i64 = value
                    .parse()
                    .map_err(|_| "Supply a whole interval in the chosen unit.")?;
                let ms = n
                    .checked_mul(multiplier)
                    .ok_or("Interval exceeds the supported millisecond range.")?;
                dpt.encode_ms(ms).map(format_hex).map_err(|e| match e {
                    CodecError::OutsideRange => {
                        let range = dpt.range_ms();
                        format!(
                            "Supply an interval from {} to {} ms.",
                            range.start(),
                            range.end()
                        )
                    }
                    _ => e.to_string(),
                })
            }
            Self::Priority(dpt) => {
                let (zero, one) = dpt.labels();
                let command = match value()? {
                    "release" => PriorityControl::Release,
                    v if v == zero => PriorityControl::Set(false),
                    v if v == one => PriorityControl::Set(true),
                    _ => return Err(format!("Supply release, {zero} or {one}.")),
                };
                Ok(format_hex(dpt.encode(command)))
            }
            Self::Dimming => {
                let command = match args {
                    [v] if v == "stop" => Dimming::Stop,
                    [dir, size] if dir == "increase" => Dimming::Increase(parse_step(size)?),
                    [dir, size] if dir == "decrease" => Dimming::Decrease(parse_step(size)?),
                    _ => return Err("Supply stop, increase FRACTION or decrease FRACTION.".into()),
                };
                Ok(format_hex(encode_dimming(command)))
            }
            Self::Blinds => {
                let command = match args {
                    [v] if v == "stop" => BlindControl::Stop,
                    [dir, size] if dir == "up" => BlindControl::Up(parse_step(size)?),
                    [dir, size] if dir == "down" => BlindControl::Down(parse_step(size)?),
                    _ => return Err("Supply stop, up FRACTION or down FRACTION.".into()),
                };
                Ok(format_hex(encode_blind_control(command)))
            }
            Self::SceneNumber => Ok(format_hex(encode_scene_number(parse_scene(value()?)?))),
            Self::SceneControl => {
                let command = match args {
                    [action, n] if action == "activate" => SceneControl::Activate(parse_scene(n)?),
                    [action, n] if action == "learn" => SceneControl::Learn(parse_scene(n)?),
                    _ => return Err("Supply activate NUMBER or learn NUMBER (1 to 64).".into()),
                };
                Ok(format_hex(encode_scene_control(command)))
            }
            Self::Switch => match value()? {
                "on" => Ok(format_hex(encode_switch(true))),
                "off" => Ok(format_hex(encode_switch(false))),
                _ => Err("Supply on or off.".into()),
            },
            Self::Angle => encode_angle_degrees(value()?.parse().map_err(|_| decimal)?)
                .map(format_hex)
                .map_err(|e| e.to_string()),
            Self::DecimalFactor => Ok(format_hex(encode_decimal_factor(
                value()?
                    .parse()
                    .map_err(|_| "Supply an integer ratio from 0 to 255.")?,
            ))),
            Self::PulseCount => Ok(format_hex(encode_pulse_count(
                value()?
                    .parse()
                    .map_err(|_| "Supply a pulse count from 0 to 255.")?,
            ))),
            Self::Tariff => {
                let tariff = match value()? {
                    "unavailable" => Tariff::UNAVAILABLE,
                    number => number
                        .parse()
                        .ok()
                        .and_then(|n| Tariff::numbered(n).ok())
                        .ok_or("Supply unavailable or a tariff number from 1 to 254.")?,
                };
                Ok(format_hex(tariff.encode()))
            }
            Self::SignedPercent => {
                Ok(format_hex(encode_percent_i8(value()?.parse().map_err(
                    |_| "Supply a whole percentage from -128 to 127.",
                )?)))
            }
            Self::SignedPulses => Ok(format_hex(encode_signed_pulse_count(
                value()?
                    .parse()
                    .map_err(|_| "Supply a whole pulse count from -128 to 127.")?,
            ))),
            Self::Status => Ok(format_hex(parse_status(args)?.encode())),
            Self::WholePercent => {
                Ok(format_hex(encode_percent_u8(value()?.parse().map_err(
                    |_| "Supply a whole percentage from 0 to 255.",
                )?)))
            }
            Self::Scaling => encode_scaling_percent(value()?.parse().map_err(|_| decimal)?)
                .map(format_hex)
                .map_err(|e| e.to_string()),
            Self::Temperature => encode_temperature_celsius(value()?.parse().map_err(|_| decimal)?)
                .map(format_hex)
                .map_err(|e| e.to_string()),
            Self::Energy32(dpt) => {
                Ok(format_hex(dpt.encode(value()?.parse().map_err(
                    |_| "Supply a signed 32-bit whole number in the commissioned unit.",
                )?)))
            }
            Self::Energy64(dpt) => {
                Ok(format_hex(dpt.encode(value()?.parse().map_err(
                    |_| "Supply a signed 64-bit whole number in the commissioned unit.",
                )?)))
            }
            Self::Power => encode_power_watts(value()?.parse().map_err(|_| decimal)?)
                .map(format_hex)
                .map_err(|e| e.to_string()),
        }
    }
}

fn format_pulses(count: i16) -> String {
    let unit = if matches!(count, -1 | 1) {
        "pulse"
    } else {
        "pulses"
    };
    format!("{count} {unit}")
}

fn format_status(status: StatusWithMode) -> String {
    let mut flags = String::new();
    for (letter, set) in ('a'..='e').zip(status.set_flags) {
        if set {
            flags.push(letter);
        }
    }
    if flags.is_empty() {
        flags.push_str("none");
    }
    let mode = match status.mode {
        StatusMode::Mode0 => 0,
        StatusMode::Mode1 => 1,
        StatusMode::Mode2 => 2,
    };
    format!("{flags} {mode}")
}

fn parse_status(args: &[String]) -> Result<StatusWithMode, &'static str> {
    let [letters, number] = args else {
        return Err("Supply set flags and a mode, for example ace 1 or none 0.");
    };
    let mode = match number.as_str() {
        "0" => StatusMode::Mode0,
        "1" => StatusMode::Mode1,
        "2" => StatusMode::Mode2,
        _ => return Err("Supply mode 0, 1 or 2."),
    };
    let mut set_flags = [false; 5];
    if letters != "none" {
        let hint = "Supply distinct set flags from a to e, or none.";
        if letters.is_empty() {
            return Err(hint);
        }
        for letter in letters.bytes() {
            if !(b'a'..=b'e').contains(&letter) {
                return Err(hint);
            }
            let flag = &mut set_flags[usize::from(letter - b'a')];
            if *flag {
                return Err(hint);
            }
            *flag = true;
        }
    }
    Ok(StatusWithMode { set_flags, mode })
}

fn parse_scene(value: &str) -> Result<SceneNumber, &'static str> {
    value
        .parse()
        .ok()
        .and_then(|n| SceneNumber::new(n).ok())
        .ok_or("Supply a scene number from 1 to 64.")
}

fn parse_step(value: &str) -> Result<StepSize, &'static str> {
    value
        .strip_prefix("1/")
        .and_then(|n| n.parse().ok())
        .and_then(|n| StepSize::from_denominator(n).ok())
        .ok_or("Supply a step of 1/1, 1/2, 1/4, 1/8, 1/16, 1/32 or 1/64.")
}

fn format_hex<const N: usize>(bytes: [u8; N]) -> String {
    bytes.map(|byte| format!("{byte:02X}")).join(" ")
}

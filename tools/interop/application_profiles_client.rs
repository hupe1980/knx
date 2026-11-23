//! Offline FB 320/321, 417/418 and 800 client for the XKNX peer in application_profiles_peer.py.
use knx::{
    Dimming, StepSize, encode_scaling_percent,
    lighting::{DimmingActuator, SwitchActuator},
    shading::{Direction, SunblindActuator},
    telegram::GroupTelegram,
    temperature::TemperatureSensor,
};
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let switch = SwitchActuator {
        command: "1/2/1".parse()?,
        feedback: Some((0x1102, "1/2/2".parse()?)),
    };
    let dimmer = DimmingActuator::new("1/2/1".parse()?, "1/2/2".parse()?, "1/2/3".parse()?)?
        .with_level_feedback(0x1102, "1/2/4".parse()?)?;
    match std::env::args().nth(1).as_deref() {
        Some("commands") => {
            let level = encode_scaling_percent(50.)?;
            let mut output = std::io::stdout().lock();
            for frame in [
                switch.set(0x1101, true),
                switch.set(0x1101, false),
                dimmer.set(0x1101, true),
                dimmer.dim(0x1101, Dimming::Increase(StepSize::from_denominator(4)?)),
                dimmer.set_level(0x1101, &level),
            ] {
                let mut bytes = [0; 16];
                let len = frame.encode(&mut bytes)?;
                output.write_all(&[len as u8])?;
                output.write_all(&bytes[..len])?;
            }
        }
        Some("shading") => {
            let blind = SunblindActuator::new("1/2/3".parse()?, "1/2/4".parse()?)?;
            let mut output = std::io::stdout().lock();
            for request in [
                blind.travel(0x1101, Direction::Up),
                blind.travel(0x1101, Direction::Down),
                blind.stop_or_step(0x1101, Direction::Up),
                blind.stop_or_step(0x1101, Direction::Down),
            ] {
                let mut bytes = [0; 11];
                let size = request.encode(&mut bytes)?;
                output.write_all(&bytes[..size])?;
            }
        }
        Some("temperature") => {
            let outside = TemperatureSensor {
                source: 0x1105,
                read_enabled: false,
                output: "2/1/1".parse()?,
            };
            let room = TemperatureSensor {
                source: 0x1106,
                read_enabled: true,
                output: "2/1/2".parse()?,
            };
            let mut wire = [0; 37]; // Two sensor indications and a group read.
            std::io::stdin().read_exact(&mut wire)?;
            assert_eq!(
                outside.observed_celsius(&GroupTelegram::parse(&wire[..13])?)?,
                Some(-12.5)
            );
            assert_eq!(
                room.observed_celsius(&GroupTelegram::parse(&wire[13..26])?)?,
                Some(21.5)
            );
            let read = GroupTelegram::parse(&wire[26..])?;
            let mut samples = [[0; 2]; 2];
            let [ref mut first, ref mut second] = samples;
            let mut buffer = [0; 13];
            let mut stdout = std::io::stdout().lock();
            for telegram in [
                outside.publish(-9., first)?,
                room.respond_to_read(&read, 20., second)?
                    .ok_or("read did not match the room sensor")?,
            ] {
                let size = telegram.encode(&mut buffer)?;
                stdout.write_all(&buffer[..size])?;
            }
        }
        Some("feedback") => {
            let mut bytes = [0; 34];
            let mut input = std::io::stdin().lock();
            input.read_exact(&mut bytes)?;
            if input.read(&mut [0])? != 0 {
                return Err("unexpected feedback data".into());
            }
            for (raw, expected) in [(&bytes[..11], true), (&bytes[11..22], false)] {
                assert_eq!(
                    switch.observed(&GroupTelegram::parse(raw)?)?,
                    Some(expected)
                );
            }
            assert_eq!(
                dimmer.observed_level(&GroupTelegram::parse(&bytes[22..])?)?,
                Some(128. * 100. / 255.)
            );
        }
        _ => return Err("select commands, shading, feedback or temperature".into()),
    }
    Ok(())
}

//! Relative controls and scenes in KNX Datapoint Types §§3.3, 3.18 and 3.19.
use crate::{CodecError, codecs::bytes};

/// Fraction of the full control range. A denominator of 4 is a quarter step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepSize(u8);

impl StepSize {
    /// Accept 1, 2, 4, 8, 16, 32 or 64 intervals across the full range.
    pub fn from_denominator(intervals: u8) -> Result<Self, CodecError> {
        if intervals.is_power_of_two() && intervals <= 64 {
            Ok(Self(intervals.trailing_zeros() as u8 + 1))
        } else {
            Err(CodecError::OutsideRange)
        }
    }

    /// Denominator of this step's exact fraction of the full range.
    pub fn denominator(self) -> u8 {
        1 << (self.0 - 1)
    }
}

/// DPT 3.007 relative brightness command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimming {
    Stop,
    Increase(StepSize),
    Decrease(StepSize),
}

/// DPT 3.008 relative blind or slat command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlindControl {
    Stop,
    Up(StepSize),
    Down(StepSize),
}

fn relative_step(payload: &[u8]) -> Result<Option<(bool, StepSize)>, CodecError> {
    let [raw] = bytes(payload)?;
    if raw > 15 {
        return Err(CodecError::OutsideRange);
    }
    let code = raw & 7;
    Ok(match code {
        0 => None,
        _ => Some((raw & 8 != 0, StepSize(code))),
    })
}

/// Decode a right-aligned four-bit dimming command. Both break codes mean stop.
pub fn decode_dimming(payload: &[u8]) -> Result<Dimming, CodecError> {
    Ok(match relative_step(payload)? {
        None => Dimming::Stop,
        Some((true, step)) => Dimming::Increase(step),
        Some((false, step)) => Dimming::Decrease(step),
    })
}

/// Encode an extracted DPT 3.007 value, using 0x00 for stop.
pub fn encode_dimming(command: Dimming) -> [u8; 1] {
    [match command {
        Dimming::Stop => 0,
        Dimming::Increase(step) => 8 | step.0,
        Dimming::Decrease(step) => step.0,
    }]
}

/// Decode a right-aligned four-bit blind command. Both break codes mean stop.
pub fn decode_blind_control(payload: &[u8]) -> Result<BlindControl, CodecError> {
    Ok(match relative_step(payload)? {
        None => BlindControl::Stop,
        Some((true, step)) => BlindControl::Down(step),
        Some((false, step)) => BlindControl::Up(step),
    })
}

/// Encode an extracted DPT 3.008 value, using 0x00 for stop.
pub fn encode_blind_control(command: BlindControl) -> [u8; 1] {
    [match command {
        BlindControl::Stop => 0,
        BlindControl::Down(step) => 8 | step.0,
        BlindControl::Up(step) => step.0,
    }]
}

/// Scene number as displayed in ETS, from 1 through 64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneNumber(u8);

impl SceneNumber {
    /// Construct a scene from its one-based display number.
    pub fn new(number: u8) -> Result<Self, CodecError> {
        if (1..=64).contains(&number) {
            Ok(Self(number))
        } else {
            Err(CodecError::OutsideRange)
        }
    }
    /// The one-based display number.
    pub fn number(self) -> u8 {
        self.0
    }
}

/// Decode a DPT 17.001 scene number with both reserved bits clear.
pub fn decode_scene_number(payload: &[u8]) -> Result<SceneNumber, CodecError> {
    let [raw] = bytes(payload)?;
    if raw > 63 {
        return Err(CodecError::OutsideRange);
    }
    Ok(SceneNumber(raw + 1))
}

/// Encode a scene number into the zero-based DPT 17.001 field.
pub fn encode_scene_number(scene: SceneNumber) -> [u8; 1] {
    [scene.0 - 1]
}

/// DPT 18.001 command to activate a scene or learn its current settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneControl {
    Activate(SceneNumber),
    Learn(SceneNumber),
}

/// Decode the scene and action, rejecting a set reserved bit.
pub fn decode_scene_control(payload: &[u8]) -> Result<SceneControl, CodecError> {
    let [raw] = bytes(payload)?;
    let scene = decode_scene_number(&[raw & 0x7f])?;
    Ok(if raw & 0x80 == 0 {
        SceneControl::Activate(scene)
    } else {
        SceneControl::Learn(scene)
    })
}

/// Encode a DPT 18.001 command with its reserved bit clear.
pub fn encode_scene_control(command: SceneControl) -> [u8; 1] {
    match command {
        SceneControl::Activate(scene) => encode_scene_number(scene),
        SceneControl::Learn(scene) => [0x80 | encode_scene_number(scene)[0]],
    }
}

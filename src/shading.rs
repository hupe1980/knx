//! Client bindings for the two mandatory inputs of a basic sunblind actuator (FB 800).
use crate::{
    GroupAddress,
    telegram::{GroupData, GroupService, GroupTelegram},
};

/// Direction as seen by the commissioned actuator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// DPA 800.81 starts travel. DPA 800.82 stops travel or steps the slats.
#[derive(Debug, Clone, Copy)]
pub struct SunblindActuator {
    travel: GroupAddress,
    step: GroupAddress,
}

impl SunblindActuator {
    /// Returns an error if the DPT 1.008 travel and DPT 1.007 step inputs share a group.
    pub fn new(travel: GroupAddress, step: GroupAddress) -> Result<Self, ConflictingInputs> {
        if travel == step {
            return Err(ConflictingInputs);
        }
        Ok(Self { travel, step })
    }

    /// Request continuous travel in the selected direction.
    pub fn travel(&self, source: u16, direction: Direction) -> GroupTelegram<'static> {
        command(source, self.travel, direction)
    }

    /// Request a step in the selected direction. A moving actuator interprets this as a stop.
    pub fn stop_or_step(&self, source: u16, direction: Direction) -> GroupTelegram<'static> {
        command(source, self.step, direction)
    }
}

fn command(source: u16, group: GroupAddress, direction: Direction) -> GroupTelegram<'static> {
    let bit = u8::from(direction == Direction::Down);
    GroupTelegram::request(source, group, GroupService::Write(GroupData::Inline(bit)))
}

/// Both mandatory sunblind inputs were assigned the same group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConflictingInputs;

impl std::fmt::Display for ConflictingInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Assign separate travel and stop/step groups.")
    }
}

impl std::error::Error for ConflictingInputs {}

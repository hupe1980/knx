//! Client bindings for basic switching (FB 417) and dimming (FB 418) actuators.
use crate::{
    CodecError, GroupAddress, decode_switch,
    telegram::{GroupData, GroupService, GroupTelegram, LinkPrimitive},
};

#[derive(Clone, Copy, Debug)]
pub struct SwitchActuator {
    /// Commissioned DPT 1.001 input of the actuator.
    pub command: GroupAddress,
    /// Commissioned actuator individual address and DPT 1.001 status output.
    pub feedback: Option<(u16, GroupAddress)>,
}

impl SwitchActuator {
    /// Construct a group write to the command object.
    pub fn set(&self, local_address: u16, on: bool) -> GroupTelegram<'static> {
        GroupTelegram::request(
            local_address,
            self.command,
            GroupService::Write(GroupData::Inline(u8::from(on))),
        )
    }

    /// Return an observed switch state only for a matching actuator indication.
    pub fn observed(&self, telegram: &GroupTelegram<'_>) -> Result<Option<bool>, CodecError> {
        if telegram.primitive != LinkPrimitive::Indication
            || self.feedback != Some((telegram.source, telegram.destination))
        {
            return Ok(None);
        }
        match telegram.indicated_value() {
            None => Ok(None),
            Some(GroupData::Inline(value)) => decode_switch(&[value]).map(Some),
            Some(GroupData::Bytes(_)) => Err(CodecError::InvalidData),
        }
    }
}

/// One commissioned group was bound to two incompatible datapoint types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConflictingGroup;

impl std::fmt::Display for ConflictingGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "Assign separate group addresses to switching, relative dimming and absolute levels.",
        )
    }
}

impl std::error::Error for ConflictingGroup {}

/// Client binding for the three mandatory inputs of a basic dimming actuator (FB 418).
///
/// DPA 418.62 accepts on/off, 418.68 a relative step, and 418.70 an absolute level.
/// DPA 418.52 can optionally report the actual level.
#[derive(Clone, Copy, Debug)]
pub struct DimmingActuator {
    switch: GroupAddress,
    relative: GroupAddress,
    absolute: GroupAddress,
    feedback: Option<(u16, GroupAddress)>,
}

impl DimmingActuator {
    /// Commission the three commands. Their distinct DPTs require distinct groups.
    pub fn new(
        switch: GroupAddress,
        relative: GroupAddress,
        absolute: GroupAddress,
    ) -> Result<Self, ConflictingGroup> {
        if switch == relative || relative == absolute || absolute == switch {
            return Err(ConflictingGroup);
        }
        Ok(Self {
            switch,
            relative,
            absolute,
            feedback: None,
        })
    }

    /// Bind actual-level feedback to an actuator and a DPT 5.001 group.
    pub fn with_level_feedback(
        mut self,
        actuator: u16,
        group: GroupAddress,
    ) -> Result<Self, ConflictingGroup> {
        if group == self.switch || group == self.relative {
            return Err(ConflictingGroup);
        }
        self.feedback = Some((actuator, group));
        Ok(self)
    }

    pub fn set(&self, source: u16, on: bool) -> GroupTelegram<'static> {
        GroupTelegram::request(
            source,
            self.switch,
            GroupService::Write(GroupData::Inline(u8::from(on))),
        )
    }

    pub fn dim(&self, source: u16, step: crate::Dimming) -> GroupTelegram<'static> {
        let [bits] = crate::encode_dimming(step);
        GroupTelegram::request(
            source,
            self.relative,
            GroupService::Write(GroupData::Inline(bits)),
        )
    }

    /// Command a DPT 5.001 level. Use [`crate::encode_scaling_percent`] for a percentage.
    pub fn set_level<'a>(&self, source: u16, level: &'a [u8; 1]) -> GroupTelegram<'a> {
        GroupTelegram::request(
            source,
            self.absolute,
            GroupService::Write(GroupData::Bytes(level)),
        )
    }

    /// Percentage reported by the commissioned actuator, including its wire quantization.
    pub fn observed_level(&self, telegram: &GroupTelegram<'_>) -> Result<Option<f64>, CodecError> {
        if self.feedback != Some((telegram.source, telegram.destination)) {
            return Ok(None);
        }
        match telegram.indicated_value() {
            None => Ok(None),
            Some(GroupData::Bytes(level)) => crate::decode_scaling_percent(level).map(Some),
            Some(GroupData::Inline(_)) => Err(CodecError::InvalidData),
        }
    }
}

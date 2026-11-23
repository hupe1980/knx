//! Mandatory measured-temperature output of a commissioned outside (FB 320)
//! or room (FB 321) temperature sensor. Both outputs carry DPT 9.001 °C.

use crate::{
    CodecError, GroupAddress, decode_temperature_celsius, encode_temperature_celsius,
    telegram::{GroupData, GroupService, GroupTelegram, LinkPrimitive},
};

/// Individual address and output group assigned to one temperature sensor.
#[derive(Clone, Copy, Debug)]
pub struct TemperatureSensor {
    pub source: u16,
    pub output: GroupAddress,
    /// Commissioned read flag on the measured-temperature output.
    pub read_enabled: bool,
}

impl TemperatureSensor {
    /// Accept a measurement from the commissioned source and output group.
    pub fn observed_celsius(
        &self,
        telegram: &GroupTelegram<'_>,
    ) -> Result<Option<f64>, CodecError> {
        if (telegram.source, telegram.destination) != (self.source, self.output) {
            return Ok(None);
        }
        match telegram.indicated_value() {
            Some(GroupData::Bytes(bytes)) => decode_temperature_celsius(bytes).map(Some),
            Some(GroupData::Inline(_)) => Err(CodecError::InvalidData),
            None => Ok(None),
        }
    }

    /// Publish a measured temperature. The returned telegram borrows the encoded octets.
    pub fn publish<'a>(
        &self,
        celsius: f64,
        payload: &'a mut [u8; 2],
    ) -> Result<GroupTelegram<'a>, CodecError> {
        *payload = encode_temperature_celsius(celsius)?;
        Ok(GroupTelegram::request(
            self.source,
            self.output,
            GroupService::Write(GroupData::Bytes(payload)),
        ))
    }

    /// Answer an incoming group read when commissioning permits reads on this output.
    pub fn respond_to_read<'a>(
        &self,
        incoming: &GroupTelegram<'_>,
        celsius: f64,
        payload: &'a mut [u8; 2],
    ) -> Result<Option<GroupTelegram<'a>>, CodecError> {
        if !self.read_enabled
            || incoming.destination != self.output
            || incoming.primitive != LinkPrimitive::Indication
            || incoming.service != GroupService::Read
        {
            return Ok(None);
        }
        *payload = encode_temperature_celsius(celsius)?;
        Ok(Some(GroupTelegram::request(
            self.source,
            self.output,
            GroupService::Response(GroupData::Bytes(payload)),
        )))
    }
}

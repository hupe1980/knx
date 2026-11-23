use crate::{CodecError, codecs::bytes};

/// Decode signed watts from four network-order application bytes for DPT 14.056.
///
/// Select the datapoint type from the commissioned configuration. The meaning of
/// the sign depends on the installation. Use [`crate::PowerMeter`] to validate a
/// site-bound consumption reading and its observation time.
///
/// ```
/// assert_eq!(knx::decode_power_watts(&[0xc6, 0xbb, 0x80, 0]), Ok(-24_000.0));
/// ```
pub fn decode_power_watts(payload: &[u8]) -> Result<f32, CodecError> {
    decode_finite(payload)
}

pub(crate) fn decode_finite(payload: &[u8]) -> Result<f32, CodecError> {
    let value = f32::from_be_bytes(bytes(payload)?);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CodecError::NonFinite)
    }
}

/// Encode finite signed watts for DPT 14.056, preserving the sign of zero.
pub fn encode_power_watts(watts: f32) -> Result<[u8; 4], CodecError> {
    encode_finite(watts)
}

pub(crate) fn encode_finite(value: f32) -> Result<[u8; 4], CodecError> {
    if value.is_finite() {
        Ok(value.to_be_bytes())
    } else {
        Err(CodecError::NonFinite)
    }
}

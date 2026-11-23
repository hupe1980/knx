//! KNX datapoint codecs, cEMI group telegrams and KNXnet/IP for Rust applications.
//!
//! Select the subtype from the commissioned configuration. Use [`GroupAddress`] to
//! match an ETS address with a transport destination, or [`PowerMeter`] to validate
//! a timestamped consumption reading. Use [`telegram`] for cEMI records and [`tunnel`]
//! for a UDP/IPv4 link-layer tunnel. Bind commissioned points with [`lighting`],
//! [`shading`] and [`temperature`].
//!
//! Enable `data-secure` for encrypted group messages, `ip-secure` for authenticated
//! TCP sessions, or `secure-routing` for authenticated multicast with an external clock
//! or built-in Timer Notify synchronization. Enable `keyring` to import ETS credentials.

mod address;
mod building;
mod codecs;
pub mod connected_transport;
mod controls;
#[cfg(feature = "data-secure")]
pub mod data_secure;
pub mod domain;
mod energy;
pub mod hvac;
pub mod hvac_flags;
pub mod hvac_reports;
pub mod iot;
pub mod ip_core;
#[cfg(feature = "ip-secure")]
pub mod ip_secure;
#[cfg(any(feature = "ip-secure", feature = "secure-routing"))]
mod ip_secure_crypto;
#[cfg(feature = "keyring")]
pub mod keyring;
pub mod lighting;
pub mod media;
pub mod parameters;
mod power;
mod priority;
mod property_description;
mod property_read;
mod protection;
pub mod registers;
pub mod routing;
mod scalars;
#[cfg(feature = "secure-routing")]
pub mod secure_routing;
pub mod shading;
mod status;
pub mod structured;
pub mod system;
pub mod telegram;
pub mod temperature;
pub mod text_time;
mod time_interval;
pub mod tunnel;

pub use address::{GroupAddress, InvalidGroupAddress};
pub use building::{DatedTariffEnergy, HvacMode, LightingActuator, LightingStatus, TariffEnergy};
pub use codecs::{
    CodecError, Tariff, decode_angle_degrees, decode_decimal_factor, decode_energy_wh,
    decode_percent_i8, decode_percent_u8, decode_pulse_count, decode_scaling_percent,
    decode_signed_pulse_count, decode_switch, decode_temperature_celsius, encode_angle_degrees,
    encode_decimal_factor, encode_energy_wh, encode_percent_i8, encode_percent_u8,
    encode_pulse_count, encode_scaling_percent, encode_signed_pulse_count, encode_switch,
    encode_temperature_celsius,
};
pub use controls::{
    BlindControl, Dimming, SceneControl, SceneNumber, StepSize, decode_blind_control,
    decode_dimming, decode_scene_control, decode_scene_number, encode_blind_control,
    encode_dimming, encode_scene_control, encode_scene_number,
};
pub use energy::{Energy32, Energy64};
pub use power::{decode_power_watts, encode_power_watts};
pub use priority::{PriorityControl, PriorityDpt};
pub use property_description::{DescriptionError, PropertyDescription, PropertyDescriptionRead};
pub use property_read::{
    InterfaceObjectList, PropertyRead, PropertyReadError, PropertyUpdate, PropertyUpdateError,
    PropertyUpdateState,
};
pub use protection::{DeliveryPolicy, ProtectionError, ProtectionEvidence, ProtectionRequirement};
pub use scalars::*;
pub use status::{StatusMode, StatusWithMode};
pub use time_interval::TimeIntervalDpt;

/// Reason a meter binding or observation was rejected. Display messages can be
/// shown to operators and contain no submitted values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MeterError {
    InvalidGroup,
    InvalidSite,
    InvalidRange,
    WrongBinding,
    WrongDpt,
    PayloadLength,
    OutsideRange,
    MissingTime,
    FutureTime,
    OutOfOrder,
}

impl std::fmt::Display for MeterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidGroup => "Check the commissioned group address.",
            Self::InvalidSite => "Supply a nonempty site identifier.",
            Self::InvalidRange => "Supply a positive finite meter range.",
            Self::WrongBinding => "Check the meter site and group binding.",
            Self::WrongDpt => "Check the meter DPT in ETS. Expected 14.056 in watts.",
            Self::PayloadLength => "Supply four application payload bytes.",
            Self::OutsideRange => "Check the meter range and consumption direction.",
            Self::MissingTime => "Supply a trusted observation timestamp.",
            Self::FutureTime => "Check the meter clock. Observation time exceeds reception time.",
            Self::OutOfOrder => "Ignore repeated or older observations.",
        })
    }
}

impl std::error::Error for MeterError {}

/// A validated active power observation from the bound meter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerReading {
    /// Measured consumption in watts.
    pub watts: f32,
    /// Trusted observation time in Unix seconds.
    pub observed_at: i64,
}

/// A meter bound to one site, group address and nonnegative consumption range.
///
/// ```
/// let mut meter = knx::PowerMeter::new("west", "2/1/3", 60_000.0)?;
/// let reading = meter.observe("west", "2/1/3", "14.056", &[0x46, 0xbb, 0x80, 0],
///                           Some(1_795_335_135), 1_795_335_136)?;
/// assert_eq!(reading.watts, 24_000.0);
/// # Ok::<(), knx::MeterError>(())
/// ```
pub struct PowerMeter<'a> {
    site: &'a str,
    group: GroupAddress,
    max_w: f32,
    last_observation: Option<i64>,
}

impl<'a> PowerMeter<'a> {
    pub fn new(
        site: &'a str,
        group: impl TryInto<GroupAddress>,
        max_w: f32,
    ) -> Result<Self, MeterError> {
        let group = group.try_into().map_err(|_| MeterError::InvalidGroup)?;
        if site.trim().is_empty() {
            return Err(MeterError::InvalidSite);
        }
        if !max_w.is_finite() || max_w <= 0.0 {
            return Err(MeterError::InvalidRange);
        }
        Ok(Self {
            site,
            group,
            max_w,
            last_observation: None,
        })
    }

    /// Decode watts from a DPT 14.056 application payload. Times are Unix seconds.
    /// Supply the trusted observation time separately. Retain the meter across reconnects.
    pub fn observe(
        &mut self,
        site: &str,
        group: impl TryInto<GroupAddress>,
        dpt: &str,
        payload: &[u8],
        observed_at: Option<i64>,
        received_at: i64,
    ) -> Result<PowerReading, MeterError> {
        if site != self.site || group.try_into().ok() != Some(self.group) {
            return Err(MeterError::WrongBinding);
        }
        if dpt != "14.056" {
            return Err(MeterError::WrongDpt);
        }
        let watts = decode_power_watts(payload).map_err(|error| match error {
            CodecError::PayloadLength { .. } => MeterError::PayloadLength,
            _ => MeterError::OutsideRange,
        })?;
        if !(0.0..=self.max_w).contains(&watts) {
            return Err(MeterError::OutsideRange);
        }
        let at = observed_at.ok_or(MeterError::MissingTime)?;
        if at > received_at {
            return Err(MeterError::FutureTime);
        }
        if self.last_observation.is_some_and(|last| at <= last) {
            return Err(MeterError::OutOfOrder);
        }
        self.last_observation = Some(at);
        Ok(PowerReading {
            watts,
            observed_at: at,
        })
    }
}

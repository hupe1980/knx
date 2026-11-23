//! Commissioned lighting, blind and metering application values.
//!
//! ```
//! use knx::domain::{CombinedPosition, PhaseQuantity};
//! let position = CombinedPosition::decode(&[255, 99, 1])?;
//! assert_eq!(position.height, Some(255)); // DPT 5.001 code for 100%
//! assert_eq!(position.slats, None);
//! let amps = PhaseQuantity::Current;
//! let payload = amps.encode([1.0, 2.5, 3.0])?;
//! assert_eq!(amps.decode(&payload)?, [1.0, 2.5, 3.0]);
//! # Ok::<(), knx::CodecError>(())
//! ```

use crate::{
    CodecError,
    codecs::bytes,
    power::{decode_finite, encode_finite},
    text_time::{DateTime, decode_date_time, encode_date_time},
};

macro_rules! wire_enum {
    ($id:literal, $name:ident { $($variant:ident = $code:literal),+ $(,)? }) => {
        #[doc = concat!("Commissioned DPT ", $id, " enumeration in KNX Interworking v02.02.01.")]
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        #[repr(u8)]
        pub enum $name { $($variant = $code),+ }
        impl $name {
            pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
                match bytes(payload)? {
                    $([$code] => Ok(Self::$variant),)+
                    _ => Err(CodecError::OutsideRange)
                }
            }
            pub const fn encode(self) -> [u8; 1] { [self as u8] }
        }
    }
}

pub(crate) use wire_enum;

wire_enum!("1.1200", MeterRole {
    Consumer = 0,
    Producer = 1,
});
wire_enum!("1.1201", EnergyDirection {
    Positive = 0,
    Negative = 1,
});
wire_enum!("20.600", LightLockBehavior {
    Off = 0,
    On = 1,
    Unchanged = 2,
    ParameterValue = 3,
    MemoryValue = 4,
    UpdatedValue = 5,
    BeforeLock = 6,
});
wire_enum!("20.601", BusPowerBehavior {
    Off = 0,
    On = 1,
    Unchanged = 2,
    ParameterValue = 3,
    BeforePowerDown = 4,
});
wire_enum!("20.603", BlinkMode {
    Disabled = 0,
    WithoutAcknowledgement = 1,
    WithAcknowledgement = 2,
});
wire_enum!("20.604", LightControlMode {
    Automatic = 0,
    Manual = 1,
});
wire_enum!("20.605", SwitchInputMode {
    OneButton = 1,
    TwoButtons = 2,
});
wire_enum!("20.606", SwitchButtonAction {
    Inactive = 0,
    Off = 1,
    On = 2,
    InvertFeedback = 3,
});
wire_enum!("20.607", DimmingInputMode {
    OneButtonToggle = 1,
    OneButtonUp = 2,
    OneButtonDown = 3,
    TwoButtons = 4,
});
wire_enum!("20.608", SwitchOnMode {
    LastActualValue = 0,
    ParameterValue = 1,
    LastAbsoluteSetpoint = 2,
});
wire_enum!("20.609", LoadTypeSet {
    Automatic = 0,
    LeadingEdge = 1,
    TrailingEdge = 2,
    SwitchingOnly = 3,
    AutomaticOnce = 4,
    CompactFluorescentLeading = 5,
    CompactFluorescentTrailing = 6,
    LedLeading = 7,
    LedTrailing = 8,
});
wire_enum!("20.610", DetectedLoad {
    Undefined = 0,
    LeadingEdge = 1,
    TrailingEdge = 2,
    DetectionFailed = 3,
    CalibrationPending = 4,
    CompactFluorescentLeading = 5,
    CompactFluorescentTrailing = 6,
    LedLeading = 7,
    LedTrailing = 8,
});
wire_enum!("20.611", ConverterTestControl {
    StartFunctionTest = 1,
    StartDurationTest = 2,
    StartPartialDurationTest = 3,
    StopTest = 4,
    ResetFunctionTestDone = 5,
    ResetDurationTestDone = 6,
});
wire_enum!("20.612", ConverterControl {
    RestoreFactorySettings = 0,
    Rest = 1,
    Inhibit = 2,
    Relight = 3,
    ResetLampTime = 4,
});
wire_enum!("20.613", ConverterDataRequest {
    Status = 1,
    TestResult = 2,
    BatteryInfo = 3,
    FunctionTestInfo = 4,
    DurationTestInfo = 5,
    PartialDurationTestInfo = 6,
    ConverterInfo = 7,
    FixedConverterInfo = 8,
});
wire_enum!("20.801", BlindExceptionBehavior {
    Up = 0,
    Down = 1,
    Unchanged = 2,
    ParameterValue = 3,
    Stop = 4,
});
wire_enum!("20.802", BlindLockBehavior {
    Up = 0,
    Down = 1,
    Unchanged = 2,
    ParameterValue = 3,
    Stop = 4,
    UpdatedValue = 5,
    BeforeLock = 6,
});
wire_enum!("20.803", BlindInputMode {
    OneButtonToggle = 1,
    OneButtonUp = 2,
    OneButtonDown = 3,
    TwoButtons = 4,
});
wire_enum!("20.804", BlindControlMode {
    Automatic = 0,
    Manual = 1,
});
wire_enum!("20.1202", GasMeasurementCondition {
    Unknown = 0,
    TemperatureConverted = 1,
    BaseConditions = 2,
    MeasurementConditions = 3,
});
wire_enum!("20.1203", BreakerStatus {
    Closed = 0,
    OpenOverload = 1,
    OpenOvervoltage = 2,
    OpenLoadShedding = 3,
    OpenByCommand = 4,
    OpenOverheatAboveMaxCurrent = 5,
    OpenOverheatBelowMaxCurrent = 6,
});
wire_enum!("20.1204", EuridisStatus {
    Inactive = 0,
    ActiveUnsecured = 1,
    ActiveSecured = 2,
});
wire_enum!("20.1206", PeakNotice {
    NoNotice = 0,
    NoticeOne = 1,
    NoticeTwo = 2,
    NoticeThree = 3,
});
wire_enum!("20.1207", PeakEvent {
    NoEvent = 0,
    EventOne = 1,
    EventTwo = 2,
    EventThree = 3,
});
wire_enum!("20.1208", TicType {
    Historical = 0,
    Standard = 1,
});
wire_enum!("20.1209", TicChannel {
    None = 0,
    HistoricalSinglePhase = 1,
    HistoricalThreePhase = 2,
    StandardSinglePhase = 3,
    StandardThreePhase = 4,
});

// DPT 20.1200 gives FF an explicit meaning outside the ordinary state codes.
wire_enum!("20.1200", BreakerValveState { Closed = 0, Open = 1, Released = 2, Invalid = 255 });

/// DPT 20.602 fade time, in tenths of a second. Use an exact supported DALI step.
pub const FADE_TENTHS: [u16; 16] = [
    0, 7, 10, 14, 20, 28, 40, 57, 80, 113, 160, 226, 320, 453, 640, 905,
];

pub fn decode_fade_tenths(payload: &[u8]) -> Result<u16, CodecError> {
    let [code] = bytes(payload)?;
    FADE_TENTHS
        .get(usize::from(code))
        .copied()
        .ok_or(CodecError::OutsideRange)
}

pub fn encode_fade_tenths(tenths: u16) -> Result<[u8; 1], CodecError> {
    FADE_TENTHS
        .binary_search(&tenths)
        .map(|index| [index as u8])
        .map_err(|_| CodecError::OutsideRange)
}

/// DPT 240.800 blind height and slat angle as DPT 5.001 scaling codes.
/// Each position is independently valid. Zero and 255 represent 0% and 100%.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct CombinedPosition {
    pub height: Option<u8>,
    pub slats: Option<u8>,
}

impl CombinedPosition {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [height, slats, valid] = bytes(payload)?;
        Ok(Self {
            height: (valid & 1 != 0).then_some(height),
            slats: (valid & 2 != 0).then_some(slats),
        })
    }
    pub fn encode(self) -> [u8; 3] {
        [
            self.height.unwrap_or(0),
            self.slats.unwrap_or(0),
            u8::from(self.height.is_some()) | u8::from(self.slats.is_some()) << 1,
        ]
    }
}

/// DPT 241.800 blind actuator status. Position validity is independent of the drive flags.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct BlindStatus {
    pub position: CombinedPosition,
    pub upper_end: bool,
    pub lower_end: bool,
    pub lower_predefined: bool,
    pub target_reached: bool,
    pub height_restricted: bool,
    pub slats_restricted: bool,
    pub weather_alarm: bool,
    pub forced: bool,
    pub locked: bool,
    pub local_override: bool,
    pub failure: bool,
}

impl BlindStatus {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [height, slats, high, low] = bytes(payload)?;
        let flags = u16::from_be_bytes([high, low]);
        Ok(Self {
            position: CombinedPosition::decode(&[height, slats, high >> 6])?,
            upper_end: flags & 1 != 0,
            lower_end: flags & 2 != 0,
            lower_predefined: flags & 4 != 0,
            target_reached: flags & 8 != 0,
            height_restricted: flags & 16 != 0,
            slats_restricted: flags & 32 != 0,
            weather_alarm: flags & 64 != 0,
            forced: flags & 128 != 0,
            locked: flags & 256 != 0,
            local_override: flags & 512 != 0,
            failure: flags & 1024 != 0,
        })
    }
    pub fn encode(self) -> [u8; 4] {
        let [height, slats, valid] = self.position.encode();
        let flags = [
            self.upper_end,
            self.lower_end,
            self.lower_predefined,
            self.target_reached,
            self.height_restricted,
            self.slats_restricted,
            self.weather_alarm,
            self.forced,
            self.locked,
            self.local_override,
            self.failure,
        ];
        let bits = flags
            .iter()
            .enumerate()
            .fold(u16::from(valid) << 14, |bits, (i, &flag)| {
                bits | (u16::from(flag) << i)
            });
        let [high, low] = bits.to_be_bytes();
        [height, slats, high, low]
    }
}

/// DPT 232.600 raw RGB channel levels. The device defines the colorimetry.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}
impl Rgb {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [red, green, blue] = bytes(payload)?;
        Ok(Self { red, green, blue })
    }
    pub const fn encode(self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }
}

/// DPT 251.600. Each RGBW channel carries a DPT 5.001 scaling code when valid.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Rgbw {
    pub red: Option<u8>,
    pub green: Option<u8>,
    pub blue: Option<u8>,
    pub white: Option<u8>,
}
impl Rgbw {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [r, g, b, w, _, mask] = bytes(payload)?;
        Ok(Self {
            red: (mask & 8 != 0).then_some(r),
            green: (mask & 4 != 0).then_some(g),
            blue: (mask & 2 != 0).then_some(b),
            white: (mask & 1 != 0).then_some(w),
        })
    }
    pub fn encode(self) -> [u8; 6] {
        let mask = u8::from(self.red.is_some()) << 3
            | u8::from(self.green.is_some()) << 2
            | u8::from(self.blue.is_some()) << 1
            | u8::from(self.white.is_some());
        [
            self.red.unwrap_or(0),
            self.green.unwrap_or(0),
            self.blue.unwrap_or(0),
            self.white.unwrap_or(0),
            0,
            mask,
        ]
    }
}

/// DPT 257.1200–1202 and 270.1200–1202, in phase order L1, L2, L3.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PhaseQuantity {
    Current,
    Potential,
    ApparentPower,
}

impl PhaseQuantity {
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Current => "A",
            Self::Potential => "V",
            Self::ApparentPower => "VA",
        }
    }
    pub fn decode(self, payload: &[u8]) -> Result<[f32; 3], CodecError> {
        let raw: [u8; 12] = bytes(payload)?;
        Ok([
            decode_finite(&raw[..4])?,
            decode_finite(&raw[4..8])?,
            decode_finite(&raw[8..])?,
        ])
    }
    pub fn encode(self, phases: [f32; 3]) -> Result<[u8; 12], CodecError> {
        let mut raw = [0; 12];
        for (out, value) in raw.as_chunks_mut::<4>().0.iter_mut().zip(phases) {
            *out = encode_finite(value)?;
        }
        Ok(raw)
    }
    pub fn decode_dated(self, payload: &[u8]) -> Result<(DateTime, [f32; 3]), CodecError> {
        let raw: [u8; 20] = bytes(payload)?;
        Ok((decode_date_time(&raw[..8])?, self.decode(&raw[8..])?))
    }
    pub fn encode_dated(
        self,
        timestamp: DateTime,
        phases: [f32; 3],
    ) -> Result<[u8; 20], CodecError> {
        let mut raw = [0; 20];
        raw[..8].copy_from_slice(&encode_date_time(timestamp)?);
        raw[8..].copy_from_slice(&self.encode(phases)?);
        Ok(raw)
    }
}

macro_rules! dated_enum {
    ($name:ident, $id:literal) => {
        impl $name {
            #[doc=concat!("Decode DPT ",$id," with the independent clock quality fields.")]
            pub fn decode_dated(payload: &[u8]) -> Result<(DateTime, Self), CodecError> {
                let raw: [u8; 9] = bytes(payload)?;
                Ok((decode_date_time(&raw[..8])?, Self::decode(&raw[8..])?))
            }
            #[doc=concat!("Encode the clock fields and DPT ",$id," status.")]
            pub fn encode_dated(self, timestamp: DateTime) -> Result<[u8; 9], CodecError> {
                let mut raw = [0; 9];
                raw[..8].copy_from_slice(&encode_date_time(timestamp)?);
                raw[8] = self.encode()[0];
                Ok(raw)
            }
        }
    };
}
dated_enum!(MeterRole, "265.1200");
dated_enum!(EnergyDirection, "265.1201");
dated_enum!(BreakerStatus, "268.1203");
dated_enum!(EuridisStatus, "268.1204");
dated_enum!(PeakNotice, "268.1206");

/// DPT 20.1205 and 268.1205 depend on the meter's PLC technology.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PlcTechnology {
    Sfsk,
    G3,
}
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PlcStatus {
    Unlocked,
    Locked,
    Registered,
    NotAssociated,
    Associated,
}
impl PlcTechnology {
    pub fn decode(self, payload: &[u8]) -> Result<PlcStatus, CodecError> {
        let [value] = bytes(payload)?;
        match (self, value) {
            (Self::Sfsk, 0) => Ok(PlcStatus::Unlocked),
            (Self::Sfsk, 1) => Ok(PlcStatus::Locked),
            (Self::Sfsk, 2) => Ok(PlcStatus::Registered),
            (Self::G3, 0) => Ok(PlcStatus::NotAssociated),
            (Self::G3, 1) => Ok(PlcStatus::Associated),
            _ => Err(CodecError::OutsideRange),
        }
    }
    pub fn encode(self, value: PlcStatus) -> Result<[u8; 1], CodecError> {
        match (self, value) {
            (Self::Sfsk, PlcStatus::Unlocked) | (Self::G3, PlcStatus::NotAssociated) => Ok([0]),
            (Self::Sfsk, PlcStatus::Locked) | (Self::G3, PlcStatus::Associated) => Ok([1]),
            (Self::Sfsk, PlcStatus::Registered) => Ok([2]),
            _ => Err(CodecError::OutsideRange),
        }
    }
    pub fn decode_dated(self, payload: &[u8]) -> Result<(DateTime, PlcStatus), CodecError> {
        let raw: [u8; 9] = bytes(payload)?;
        Ok((decode_date_time(&raw[..8])?, self.decode(&raw[8..])?))
    }
    pub fn encode_dated(
        self,
        timestamp: DateTime,
        value: PlcStatus,
    ) -> Result<[u8; 9], CodecError> {
        let mut raw = [0; 9];
        raw[..8].copy_from_slice(&encode_date_time(timestamp)?);
        raw[8] = self.encode(value)?[0];
        Ok(raw)
    }
}

/// DPT 21.1200. Positions in the array follow contact bit numbers 0 through 7.
pub fn decode_contacts(payload: &[u8]) -> Result<[bool; 8], CodecError> {
    let [bits] = bytes(payload)?;
    Ok(std::array::from_fn(|i| bits & (1 << i) != 0))
}
pub fn encode_contacts(contacts: [bool; 8]) -> [u8; 1] {
    [contacts
        .iter()
        .enumerate()
        .fold(0, |bits, (i, &closed)| bits | u8::from(closed) << i)]
}
/// DPT 21.1201. Phase presence in L1, L2, L3 order.
pub fn decode_phase_presence(payload: &[u8]) -> Result<[bool; 3], CodecError> {
    let [bits] = bytes(payload)?;
    Ok([bits & 1 != 0, bits & 2 != 0, bits & 4 != 0])
}
pub fn encode_phase_presence(present: [bool; 3]) -> [u8; 1] {
    let [a, b, c] = present;
    [u8::from(a) | u8::from(b) << 1 | u8::from(c) << 2]
}

/// Forecast in the commissioned physical unit. Probability uses DPT 5.001 codes.
/// A missing delay means the source supplied no start time.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Forecast {
    pub delay_minutes: Option<std::num::NonZeroU16>,
    pub probability: Option<u8>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
}

impl Forecast {
    fn decode(
        mask: u8,
        delay: [u8; 2],
        probability: u8,
        max: Result<f64, CodecError>,
        min: Result<f64, CodecError>,
    ) -> Result<Self, CodecError> {
        let delay_minutes = if mask & 8 == 0 {
            None
        } else {
            Some(
                std::num::NonZeroU16::new(u16::from_be_bytes(delay))
                    .ok_or(CodecError::OutsideRange)?,
            )
        };
        let maximum = (mask & 2 != 0).then_some(max).transpose()?;
        let minimum = (mask & 1 != 0).then_some(min).transpose()?;
        if let (Some(maximum), Some(minimum)) = (maximum, minimum)
            && maximum <= minimum
        {
            return Err(CodecError::OutsideRange);
        }
        Ok(Self {
            delay_minutes,
            probability: (mask & 4 != 0).then_some(probability),
            maximum,
            minimum,
        })
    }
    fn header(self) -> [u8; 4] {
        let mask = u8::from(self.delay_minutes.is_some()) << 3
            | u8::from(self.probability.is_some()) << 2
            | u8::from(self.maximum.is_some()) << 1
            | u8::from(self.minimum.is_some());
        let [a, b] = self
            .delay_minutes
            .map_or(0, std::num::NonZeroU16::get)
            .to_be_bytes();
        [mask, a, b, self.probability.unwrap_or(0)]
    }
}

/// Forecasts DPT 273.001–007. This edition assigns both humidity forecasts the DPT 9.007 percentage representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherDpt(crate::Float16Dpt);
impl WeatherDpt {
    pub fn from_dpt(id: &str) -> Option<Self> {
        let scalar = match id {
            "273.001" => "9.001",
            "273.002" => "9.005",
            "273.003" | "273.004" => "9.007",
            "273.005" => "9.008",
            "273.006" => "9.030",
            "273.007" => "9.022",
            _ => return None,
        };
        crate::Float16Dpt::from_dpt(scalar).map(Self)
    }
    pub const fn unit(self) -> &'static str {
        self.0.unit()
    }
    pub fn decode(self, payload: &[u8]) -> Result<Forecast, CodecError> {
        let [mask, a, b, p, x, y, z, w] = bytes(payload)?;
        Forecast::decode(
            mask,
            [a, b],
            p,
            self.0.decode(&[x, y]),
            self.0.decode(&[z, w]),
        )
    }
    pub fn encode(self, forecast: Forecast) -> Result<[u8; 8], CodecError> {
        let [m, a, b, p] = forecast.header();
        let [x, y] = forecast
            .maximum
            .map(|v| self.0.encode(v))
            .transpose()?
            .unwrap_or_default();
        let [z, w] = forecast
            .minimum
            .map(|v| self.0.encode(v))
            .transpose()?
            .unwrap_or_default();
        let raw = [m, a, b, p, x, y, z, w];
        // The two values must still be ordered after wire quantization.
        self.decode(&raw)?;
        Ok(raw)
    }
}

/// Decode a DPT 274.001 forecast of minimum/maximum wind direction in degrees.
pub fn decode_wind_forecast(payload: &[u8]) -> Result<Forecast, CodecError> {
    let [mask, a, b, p, max, min] = bytes(payload)?;
    Forecast::decode(
        mask,
        [a, b],
        p,
        crate::decode_angle_degrees(&[max]),
        crate::decode_angle_degrees(&[min]),
    )
}
pub fn encode_wind_forecast(forecast: Forecast) -> Result<[u8; 6], CodecError> {
    let [mask, a, b, p] = forecast.header();
    let [max] = forecast
        .maximum
        .map(crate::encode_angle_degrees)
        .transpose()?
        .unwrap_or_default();
    let [min] = forecast
        .minimum
        .map(crate::encode_angle_degrees)
        .transpose()?
        .unwrap_or_default();
    let raw = [mask, a, b, p, max, min];
    decode_wind_forecast(&raw)?;
    Ok(raw)
}

/// DPT 242.600. The x/y coordinates are normalized to 0–1. Brightness uses a DPT 5.001 scaling code.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct XyColor {
    pub coordinates: Option<(f64, f64)>,
    pub brightness: Option<u8>,
}
impl XyColor {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [xh, xl, yh, yl, level, mask] = bytes(payload)?;
        let normalized = |pair| f64::from(u16::from_be_bytes(pair)) / 65535.;
        Ok(Self {
            coordinates: (mask & 2 != 0).then_some((normalized([xh, xl]), normalized([yh, yl]))),
            brightness: (mask & 1 != 0).then_some(level),
        })
    }
    pub fn encode(self) -> Result<[u8; 6], CodecError> {
        let (x, y) = self.coordinates.unwrap_or((0., 0.));
        let coord = |value: f64| {
            if !(0.0..=1.0).contains(&value) {
                return Err(CodecError::OutsideRange);
            }
            Ok(((value * 65535.0).round() as u16).to_be_bytes())
        };
        let [xh, xl] = coord(x)?;
        let [yh, yl] = coord(y)?;
        Ok([
            xh,
            xl,
            yh,
            yl,
            self.brightness.unwrap_or(0),
            u8::from(self.coordinates.is_some()) << 1 | u8::from(self.brightness.is_some()),
        ])
    }
    /// DPT 243.600 uses 100 ms ticks, and carries y before x.
    pub fn decode_transition(payload: &[u8]) -> Result<(u16, Self), CodecError> {
        let [th, tl, yh, yl, xh, xl, level, mask] = bytes(payload)?;
        Ok((
            u16::from_be_bytes([th, tl]),
            Self::decode(&[xh, xl, yh, yl, level, mask])?,
        ))
    }
    pub fn encode_transition(self, ticks_100ms: u16) -> Result<[u8; 8], CodecError> {
        let [th, tl] = ticks_100ms.to_be_bytes();
        let [xh, xl, yh, yl, level, mask] = self.encode()?;
        Ok([th, tl, yh, yl, xh, xl, level, mask])
    }
}

/// DPT 249.600. Time, colour temperature and brightness have independent validity bits.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct WhiteTransition {
    pub ticks_100ms: Option<u16>,
    pub kelvin: Option<u16>,
    pub brightness: Option<u8>,
}
impl WhiteTransition {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [th, tl, kh, kl, b, mask] = bytes(payload)?;
        Ok(Self {
            ticks_100ms: (mask & 4 != 0).then_some(u16::from_be_bytes([th, tl])),
            kelvin: (mask & 2 != 0).then_some(u16::from_be_bytes([kh, kl])),
            brightness: (mask & 1 != 0).then_some(b),
        })
    }
    pub fn encode(self) -> [u8; 6] {
        let [th, tl] = self.ticks_100ms.unwrap_or(0).to_be_bytes();
        let [kh, kl] = self.kelvin.unwrap_or(0).to_be_bytes();
        let mask = u8::from(self.ticks_100ms.is_some()) << 2
            | u8::from(self.kelvin.is_some()) << 1
            | u8::from(self.brightness.is_some());
        [th, tl, kh, kl, self.brightness.unwrap_or(0), mask]
    }
}

macro_rules! masked_steps {
    ($name:ident,$id:literal,$width:literal, $(($channel:ident,$index:literal,$bit:literal)),+) => {
        #[doc=concat!("DPT ",$id,". Each valid channel can request a relative step or stop. An absent channel leaves its ongoing transition alone.")]
        #[derive(Debug,Clone,Copy,PartialEq,Eq,Default)]
        pub struct $name { $(pub $channel:Option<crate::Dimming>),+ }
        impl $name {
            pub fn decode(payload:&[u8])->Result<Self,CodecError> {
                let raw:[u8;$width]=bytes(payload)?;
                let mask=raw[$width-1];
                Ok(Self{ $($channel:if mask & (1<<$bit)!=0 {Some(crate::decode_dimming(&[raw[$index]&15])?)}else{None}),+ })
            }
            pub fn encode(self)->[u8;$width]{
                let mut raw=[0;$width];
                $(if let Some(step)=self.$channel {raw[$index]=crate::encode_dimming(step)[0]; raw[$width-1]|=1<<$bit;})+
                raw
            }
        }
    };
}
masked_steps!(
    WhiteControl,
    "250.600",
    3,
    (temperature, 0, 1),
    (brightness, 1, 0)
);
masked_steps!(
    RgbwControl,
    "252.600",
    5,
    (red, 0, 3),
    (green, 1, 2),
    (blue, 2, 1),
    (white, 3, 0)
);
masked_steps!(
    XyControl,
    "253.600",
    4,
    (saturation, 0, 2),
    (wavelength, 1, 1),
    (brightness, 2, 0)
);

/// DPT 254.600. All three relative commands are present in RGB order.
pub fn decode_rgb_steps(payload: &[u8]) -> Result<[crate::Dimming; 3], CodecError> {
    let [r, g, b] = bytes(payload)?;
    Ok([
        crate::decode_dimming(&[r & 15])?,
        crate::decode_dimming(&[g & 15])?,
        crate::decode_dimming(&[b & 15])?,
    ])
}
pub fn encode_rgb_steps(steps: [crate::Dimming; 3]) -> [u8; 3] {
    steps.map(|step| crate::encode_dimming(step)[0])
}

pub mod dali;

/// DPT 276.1200 ERL channel duty cycles use DPT 5.001 codes in F1, F2, F3 order.
/// Alarm bits are the source's observations and remain independent of those levels.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct ErlStatus {
    pub duty_cycles: [u8; 3],
    pub duty_cycle_alarms: [bool; 3],
    pub software_updating: bool,
    pub tic_alarm: bool,
}
impl ErlStatus {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [a, b, c, flags] = bytes(payload)?;
        Ok(Self {
            duty_cycles: [a, b, c],
            duty_cycle_alarms: decode_phase_presence(&[flags])?,
            software_updating: flags & 8 != 0,
            tic_alarm: flags & 16 != 0,
        })
    }
    pub fn encode(self) -> [u8; 4] {
        let [a, b, c] = self.duty_cycles;
        let [alarms] = encode_phase_presence(self.duty_cycle_alarms);
        [
            a,
            b,
            c,
            alarms | u8::from(self.software_updating) << 3 | u8::from(self.tic_alarm) << 4,
        ]
    }
}

/// DPT 271.1200 dry-contact instruction.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ContactAction {
    Keep,
    TempoConfiguration,
    Open,
    Close,
}

/// A DPT 271.1200 tariff day-profile item. Contact slots run from VC1 through VC7.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TariffDayItem {
    pub time: crate::text_time::TimeOfDay,
    pub item: u8,
    pub total: u8,
    pub tariff: crate::Tariff,
    pub profile: u8,
    pub contact: ContactAction,
    pub virtual_contacts: [bool; 7],
}
impl TariffDayItem {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [h, m, s, n, t, p, v] = bytes(payload)?;
        let contact = match p & 3 {
            0 => ContactAction::Keep,
            1 => ContactAction::TempoConfiguration,
            2 => ContactAction::Open,
            _ => ContactAction::Close,
        };
        Ok(Self {
            time: crate::text_time::TimeOfDay::decode(&[h, m, s])?,
            item: n >> 4,
            total: n & 15,
            tariff: crate::Tariff::decode(&[t])?,
            profile: p >> 2,
            contact,
            virtual_contacts: std::array::from_fn(|i| v & (1 << i) != 0),
        })
    }
    pub fn encode(self) -> Result<[u8; 7], CodecError> {
        if self.item > 15 || self.total > 15 || self.profile > 63 {
            return Err(CodecError::OutsideRange);
        }
        let [h, m, s] = self.time.encode()?;
        let v = self
            .virtual_contacts
            .into_iter()
            .enumerate()
            .fold(0, |bits, (i, v)| bits | u8::from(v) << i);
        Ok([
            h,
            m,
            s,
            self.item << 4 | self.total,
            self.tariff.encode()[0],
            self.profile << 2 | self.contact as u8,
            v,
        ])
    }
}

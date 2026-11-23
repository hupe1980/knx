//! System properties and device configuration values from KNX Interworking §8.
//!
//! Select the DPT specified by the interface object or commissioning profile.
//! A push-button function configures a button; decoding it issues no command.
//! DPT 230.1000 preserves the raw M-Bus address and exposes a manufacturer FLAG ID when encoded.

use crate::{CodecError, codecs::bytes, domain::wire_enum};

wire_enum!("20.1000", CommunicationMode {
    LinkLayer = 0, BusMonitor = 1, RawFrames = 2, TransportLayer = 6, NoLayer = 255,
});
wire_enum!("20.1001", AdditionalInfoType {
    PowerlineDomain = 1, RadioInfo = 2, BusMonitorFlags = 3, RelativeTimestamp = 4,
    TimeDelay = 5, ExtendedRelativeTimestamp = 6, BiBat = 7,
});
wire_enum!("20.1002", RadioMode { Asynchronous = 0, BiBatMaster = 1, BiBatSlave = 2 });
wire_enum!("20.1003", RadioFilter {
    None = 0, Domain = 1, SerialNumber = 2, DomainAndSerialNumber = 3,
});
wire_enum!("20.1004", MediumType { TwistedPair = 0, Powerline = 1, Radio = 2, Ip = 5 });
wire_enum!("20.1005", ButtonFunction {
    Default = 1, On = 2, Off = 3, Toggle = 4, DimUpDown = 5, DimUp = 6, DimDown = 7,
    OnOff = 8, TimedOnOff = 9, ForceOn = 10, ForceOff = 11,
    ShutterUpButton = 12, ShutterDownButton = 13, ShutterUpDownButton = 14,
    ForceUp = 16, ForceDown = 17, WindAlarm = 18, RainAlarm = 19,
    ComfortEconomy = 20, Comfort = 21, Economy = 22, BuildingProtectionAuto = 23,
    ShutterStop = 24, TimedComfortStandby = 25, ForceComfort = 26, ForceBuildingProtection = 27,
    Scene1 = 28, Scene2 = 29, Scene3 = 30, Scene4 = 31, Scene5 = 32, Scene6 = 33,
    Scene7 = 34, Scene8 = 35, Dim25Percent = 36, Dim50Percent = 37, Dim75Percent = 38,
    Dim100Percent = 39, ShutterUpSwitch = 40, ShutterDownSwitch = 41, ShutterUpDownSwitch = 42,
    ShutterDownUpSwitch = 43, LightSensor = 44, SystemClock = 45, BatteryStatus = 46,
    Standby = 47, Automatic = 48, ComfortStandby = 49, BuildingProtection = 50,
    TimedToggle = 51, AbsoluteDimmingSwitch = 52, SceneSwitch = 53, SmokeAlarm = 54, SubDetector = 55,
});

// The required mask encodes fixed-one fields such as asynchronous RF support.
macro_rules! flag_fields {
    ($id:literal, $name:ident, $word:ty, $required:literal; $($field:ident = $bit:literal $(=> $default:literal)?),+ $(,)?) => {
        #[doc = concat!("DPT ", $id, " named bit fields in network byte order.")]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: bool,)+ }
        impl Default for $name {
            fn default() -> Self {
                Self { $($field: flag_fields!(@default $($default)?),)+ }
            }
        }
        impl $name {
            pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
                let bits = <$word>::from_be_bytes(bytes(payload)?);
                let allowed: $word = $required $(| (1 << $bit))+;
                if bits & !allowed != 0 || bits & $required != $required {
                    return Err(CodecError::OutsideRange);
                }
                Ok(Self { $($field: bits & (1 << $bit) != 0,)+ })
            }
            pub fn encode(self) -> [u8; size_of::<$word>()] {
                let bits: $word = $required $(| (<$word>::from(self.$field) << $bit))+;
                bits.to_be_bytes()
            }
        }
    };
    (@default) => { false };
    (@default $value:literal) => { $value };
}
pub(crate) use flag_fields;

flag_fields!("21.1000", RadioModeSupport, u8, 1; bibat_master=1,bibat_slave=2);
flag_fields!("21.1001", RadioFilterSupport, u8, 0; domain=0,serial_number=1,domain_and_serial_number=2);
flag_fields!("21.1002", SecurityReport, u8, 0; failure_since_previous_report=0);
flag_fields!("22.1000", SupportedMedia, u16, 0; twisted_pair=1,powerline=2,radio=4,ip=5);

/// DPT 21.1010. Bit zero identifies channel one.
pub fn encode_channels_8(active: u8) -> [u8; 1] {
    [active]
}
pub fn decode_channels_8(payload: &[u8]) -> Result<u8, CodecError> {
    Ok(u8::from_be_bytes(bytes(payload)?))
}
/// DPT 22.1010. Bit fifteen identifies channel sixteen.
pub fn encode_channels_16(active: u16) -> [u8; 2] {
    active.to_be_bytes()
}
pub fn decode_channels_16(payload: &[u8]) -> Result<u16, CodecError> {
    Ok(u16::from_be_bytes(bytes(payload)?))
}
/// DPT 30.1010. Reject any set bit above channel twenty-four.
pub fn encode_channels_24(active: u32) -> Result<[u8; 3], CodecError> {
    let [zero, a, b, c] = active.to_be_bytes();
    if zero != 0 {
        return Err(CodecError::OutsideRange);
    }
    Ok([a, b, c])
}
pub fn decode_channels_24(payload: &[u8]) -> Result<u32, CodecError> {
    let [a, b, c] = bytes(payload)?;
    Ok(u32::from_be_bytes([0, a, b, c]))
}

/// DPT 25.1000 repetition counters, each within 0–3.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Repetitions {
    pub busy: u8,
    pub negative_ack: u8,
}
impl Repetitions {
    pub fn encode(self) -> Result<[u8; 1], CodecError> {
        if self.busy > 3 || self.negative_ack > 3 {
            return Err(CodecError::OutsideRange);
        }
        Ok([self.busy << 4 | self.negative_ack])
    }
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [b] = bytes(payload)?;
        if b & 0xcc != 0 {
            return Err(CodecError::OutsideRange);
        }
        Ok(Self {
            busy: b >> 4,
            negative_ack: b & 15,
        })
    }
}

/// Three-letter M-Bus manufacturer code for lookup in the DLMS UA FLAG registry.
///
/// ```
/// use knx::system::ManufacturerFlag;
/// let flag: ManufacturerFlag = "kAm".parse()?;
/// assert_eq!(flag.as_str(), "KAM");
/// assert_eq!(u16::from(flag), 0x2c2d);
/// # Ok::<(), knx::CodecError>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ManufacturerFlag([u8; 3]);

impl ManufacturerFlag {
    /// Uppercase registry key. Assignment to a company is maintained by DLMS UA.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("validated uppercase ASCII")
    }
}

impl std::str::FromStr for ManufacturerFlag {
    type Err = CodecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut letters: [u8; 3] = s
            .as_bytes()
            .try_into()
            .map_err(|_| CodecError::InvalidData)?;
        if !letters.iter().all(u8::is_ascii_alphabetic) {
            return Err(CodecError::InvalidData);
        }
        letters.make_ascii_uppercase();
        Ok(Self(letters))
    }
}

impl TryFrom<u16> for ManufacturerFlag {
    type Error = CodecError;

    fn try_from(code: u16) -> Result<Self, Self::Error> {
        let digits = [
            (code >> 10) as u8,
            ((code >> 5) & 31) as u8,
            (code & 31) as u8,
        ];
        if !digits.iter().all(|digit| (1..=26).contains(digit)) {
            return Err(CodecError::InvalidData);
        }
        Ok(Self(digits.map(|digit| b'@' + digit)))
    }
}

impl From<ManufacturerFlag> for u16 {
    fn from(flag: ManufacturerFlag) -> Self {
        let [a, b, c] = flag.0.map(|letter| u16::from(letter - b'@'));
        a << 10 | b << 5 | c
    }
}

impl std::fmt::Display for ManufacturerFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// DPT 230.1000. Identification is manufacturer-specific and retained bit for bit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MBusAddress {
    pub manufacturer: u16,
    pub identification: u32,
    pub version: u8,
    /// M-Bus device type from the commissioning profile.
    pub medium: u8,
}
impl MBusAddress {
    /// Interpret the manufacturer field as a three-letter FLAG ID when possible.
    pub fn manufacturer_flag(self) -> Option<ManufacturerFlag> {
        self.manufacturer.try_into().ok()
    }

    pub fn encode(self) -> [u8; 8] {
        let [mh, ml] = self.manufacturer.to_be_bytes();
        let [a, b, c, d] = self.identification.to_be_bytes();
        [mh, ml, a, b, c, d, self.version, self.medium]
    }
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [mh, ml, a, b, c, d, version, medium] = bytes(payload)?;
        Ok(Self {
            manufacturer: u16::from_be_bytes([mh, ml]),
            identification: u32::from_be_bytes([a, b, c, d]),
            version,
            medium,
        })
    }
}

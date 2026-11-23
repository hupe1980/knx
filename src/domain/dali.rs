//! Standard KNX lighting diagnostics. These codecs select the §6 assignments and wire lengths.

use crate::{
    CodecError,
    codecs::bytes,
    text_time::{DateTime, decode_clq_date_time, encode_clq_date_time},
};

/// DPT 21.601. A set bit denotes an observed fault.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct LightFaults {
    pub detection: bool,
    pub undervoltage: bool,
    pub overcurrent: bool,
    pub underload: bool,
    pub defective_load: bool,
    pub lamp_failure: bool,
    pub overheat: bool,
}
impl LightFaults {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [x] = bytes(payload)?;
        Ok(Self {
            detection: x & 1 != 0,
            undervoltage: x & 2 != 0,
            overcurrent: x & 4 != 0,
            underload: x & 8 != 0,
            defective_load: x & 16 != 0,
            lamp_failure: x & 32 != 0,
            overheat: x & 64 != 0,
        })
    }
    pub fn encode(self) -> [u8; 1] {
        [u8::from(self.detection)
            | u8::from(self.undervoltage) << 1
            | u8::from(self.overcurrent) << 2
            | u8::from(self.underload) << 3
            | u8::from(self.defective_load) << 4
            | u8::from(self.lamp_failure) << 5
            | u8::from(self.overheat) << 6]
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DaliAddress {
    Device(u8),
    Group(u8),
}
impl DaliAddress {
    fn encode(self) -> Result<u8, CodecError> {
        match self {
            Self::Device(n) if n < 64 => Ok(n),
            Self::Group(n) if n < 16 => Ok(64 | n),
            _ => Err(CodecError::OutsideRange),
        }
    }
}

/// DPT 237.600 diagnostic status or request. A converter error applies to emergency lighting.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct GearDiagnostic {
    pub address: DaliAddress,
    pub read_request: bool,
    pub lamp_failure: bool,
    pub ballast_failure: bool,
    pub converter_error: bool,
}
impl GearDiagnostic {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [flags, target] = bytes(payload)?;
        let n = target & 63;
        let address = if target & 64 == 0 {
            DaliAddress::Device(n)
        } else if n < 16 {
            DaliAddress::Group(n)
        } else {
            return Err(CodecError::OutsideRange);
        };
        Ok(Self {
            address,
            read_request: target & 128 != 0,
            lamp_failure: flags & 1 != 0,
            ballast_failure: flags & 2 != 0,
            converter_error: flags & 4 != 0,
        })
    }
    pub fn encode(self) -> Result<[u8; 2], CodecError> {
        Ok([
            u8::from(self.lamp_failure)
                | u8::from(self.ballast_failure) << 1
                | u8::from(self.converter_error) << 2,
            self.address.encode()? | u8::from(self.read_request) << 7,
        ])
    }
}

/// DPT 238.600 reports faults for one DALI short address.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct DaliDiagnostic {
    pub short_address: u8,
    pub lamp_failure: bool,
    pub ballast_failure: bool,
}
impl DaliDiagnostic {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [x] = bytes(payload)?;
        Ok(Self {
            short_address: x & 63,
            lamp_failure: x & 64 != 0,
            ballast_failure: x & 128 != 0,
        })
    }
    pub fn encode(self) -> Result<[u8; 1], CodecError> {
        Ok([DaliAddress::Device(self.short_address).encode()?
            | u8::from(self.lamp_failure) << 6
            | u8::from(self.ballast_failure) << 7])
    }
}

/// Standard DPT 246.600. Charge 0–254 spans deep discharge through full. None is unknown.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct BatteryInfo {
    pub failed: bool,
    pub duration_failure: bool,
    pub fully_charged: bool,
    pub charge: Option<u8>,
}
fn level(raw: u8) -> Option<u8> {
    (raw != 255).then_some(raw)
}
fn pack_level(value: Option<u8>) -> Result<u8, CodecError> {
    if value == Some(255) {
        return Err(CodecError::OutsideRange);
    }
    Ok(value.unwrap_or(255))
}
impl BatteryInfo {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [x, charge] = bytes(payload)?;
        Ok(Self {
            failed: x & 1 != 0,
            duration_failure: x & 2 != 0,
            fully_charged: x & 4 != 0,
            charge: level(charge),
        })
    }
    pub fn encode(self) -> Result<[u8; 2], CodecError> {
        Ok([
            u8::from(self.failed)
                | u8::from(self.duration_failure) << 1
                | u8::from(self.fully_charged) << 2,
            pack_level(self.charge)?,
        ])
    }
}

/// DPT 247.600 timestamps use CLQ with seven reserved bits. Zero interval disables automatic testing.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ConverterTestInfo {
    pub previous_start: DateTime,
    pub interval_hours: u16,
    pub previous_seconds: u16,
}
impl ConverterTestInfo {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let raw: [u8; 12] = bytes(payload)?;
        Ok(Self {
            previous_start: decode_clq_date_time(&raw[..8])?,
            interval_hours: u16::from_be_bytes([raw[8], raw[9]]),
            previous_seconds: u16::from_be_bytes([raw[10], raw[11]]),
        })
    }
    pub fn encode(self) -> Result<[u8; 12], CodecError> {
        let t = encode_clq_date_time(self.previous_start)?;
        let mut out = [0; 12];
        out[..8].copy_from_slice(&t);
        out[8..10].copy_from_slice(&self.interval_hours.to_be_bytes());
        out[10..].copy_from_slice(&self.previous_seconds.to_be_bytes());
        Ok(out)
    }
}

/// Information validity for DPT 248.600 and 272.600. Unavailable covers a missing converter or information.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ConverterInfo<T> {
    Unavailable,
    Invalid,
    Valid(T),
}

impl<T> ConverterInfo<T> {
    fn unpack(code: u8, f: impl FnOnce() -> Result<T, CodecError>) -> Result<Self, CodecError> {
        match code {
            0 => Ok(Self::Unavailable),
            1 => f().map(Self::Valid),
            2 => Ok(Self::Invalid),
            _ => Err(CodecError::OutsideRange),
        }
    }
    fn code(&self) -> u8 {
        match self {
            Self::Unavailable => 0,
            Self::Valid(_) => 1,
            Self::Invalid => 2,
        }
    }
}
/// DPT 272.600 dynamic information.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ConverterRuntime {
    pub emergency_hours: u16,
    pub operating_hours: u16,
    pub emergency_level: Option<u8>,
    pub short_address: u8,
}
impl ConverterRuntime {
    pub fn decode(payload: &[u8]) -> Result<ConverterInfo<Self>, CodecError> {
        let [status, eh, el, th, tl, brightness, addr] = bytes(payload)?;
        ConverterInfo::unpack(status, || {
            DaliAddress::Device(addr).encode()?;
            Ok(Self {
                emergency_hours: u16::from_be_bytes([eh, el]),
                operating_hours: u16::from_be_bytes([th, tl]),
                emergency_level: level(brightness),
                short_address: addr,
            })
        })
    }
    pub fn encode(info: ConverterInfo<Self>) -> Result<[u8; 7], CodecError> {
        let mut raw = [0; 7];
        raw[0] = info.code();
        if let ConverterInfo::Valid(value) = info {
            raw[1..3].copy_from_slice(&value.emergency_hours.to_be_bytes());
            raw[3..5].copy_from_slice(&value.operating_hours.to_be_bytes());
            raw[5] = pack_level(value.emergency_level)?;
            raw[6] = DaliAddress::Device(value.short_address).encode()?;
        }
        Ok(raw)
    }
}

/// DPT 248.600 gear feature bits in order: integral emergency, maintained,
/// switched maintained, autotest, adjustable level, hardwired inhibit, physical selection, relight in rest.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ConverterFixed {
    pub minimum: Option<u8>,
    pub maximum: Option<u8>,
    pub rated_duration_code: u8,
    pub features: [bool; 8],
}
impl ConverterFixed {
    pub fn decode(payload: &[u8]) -> Result<ConverterInfo<Self>, CodecError> {
        let [status, min, max, rated, bits] = bytes(payload)?;
        ConverterInfo::unpack(status, || {
            Ok(Self {
                minimum: level(min),
                maximum: level(max),
                rated_duration_code: rated,
                features: super::decode_contacts(&[bits])?,
            })
        })
    }
    pub fn encode(info: ConverterInfo<Self>) -> Result<[u8; 5], CodecError> {
        let mut raw = [0; 5];
        raw[0] = info.code();
        if let ConverterInfo::Valid(value) = info {
            raw[1] = pack_level(value.minimum)?;
            raw[2] = pack_level(value.maximum)?;
            raw[3] = value.rated_duration_code;
            raw[4] = super::encode_contacts(value.features)[0];
        }
        Ok(raw)
    }
}

macro_rules! field_enum {
    ($name:ident {$($field:ident=$n:literal),+})=>{
        #[derive(Debug,Copy,Clone,Eq,PartialEq)]
        #[repr(u8)]
        pub enum $name { $($field=$n),+ }
        impl TryFrom<u8> for $name {
            type Error=CodecError;
            fn try_from(raw:u8)->Result<Self,CodecError>{
                match raw {$($n=>Ok(Self::$field),)+ _=>Err(CodecError::OutsideRange)}
            }
        }
    };
}
field_enum!(ConverterMode {Unknown=0, Normal=1, Inhibit=2, HardwiredInhibit=3, Rest=4, Emergency=5, ExtendedEmergency=6, FunctionTest=7, DurationTest=8, PartialDurationTest=9});
field_enum!(TestPending {Unknown=0, Clear=1, Pending=2});
field_enum!(Failure {Unknown=0, Clear=1, Detected=2});

/// DPT 244.600. Test slots are function, duration and partial duration.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ConverterStatus {
    pub mode: ConverterMode,
    pub hardwired_inhibit: bool,
    pub hardwired_switch: bool,
    pub pending: [TestPending; 3],
    pub failure: Failure,
}
impl ConverterStatus {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [mode, flags] = bytes(payload)?;
        Ok(Self {
            mode: (mode >> 4).try_into()?,
            hardwired_inhibit: mode & 1 != 0,
            hardwired_switch: mode & 2 != 0,
            pending: [
                (flags >> 6).try_into()?,
                ((flags >> 4) & 3).try_into()?,
                ((flags >> 2) & 3).try_into()?,
            ],
            failure: (flags & 3).try_into()?,
        })
    }
    pub fn encode(self) -> [u8; 2] {
        let [f, d, p] = self.pending;
        [
            (self.mode as u8) << 4
                | u8::from(self.hardwired_switch) << 1
                | u8::from(self.hardwired_inhibit),
            (f as u8) << 6 | (d as u8) << 4 | (p as u8) << 2 | self.failure as u8,
        ]
    }
}
field_enum!(LastTestResult {Unknown=0, PassedOnTime=1, PassedLate=2, FailedOnTime=3, FailedLate=4, StoppedManually=5});
field_enum!(TestStart {Unknown=0, Automatic=1, Gateway=2});

/// DPT 245.600. Test slots are function, duration and partial duration.
/// Discharge duration 510 means 510 minutes or longer. Charge 255 means unknown.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ConverterResults {
    pub last: [LastTestResult; 3],
    pub started_by: [TestStart; 3],
    pub discharge_minutes: u16,
    pub remaining_charge: Option<u8>,
}
impl ConverterResults {
    pub fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let [fd, p, methods, h, l, charge] = bytes(payload)?;
        let discharge_minutes = u16::from_be_bytes([h, l]);
        if discharge_minutes > 510 {
            return Err(CodecError::OutsideRange);
        }
        Ok(Self {
            last: [
                (fd >> 4).try_into()?,
                (fd & 15).try_into()?,
                (p >> 4).try_into()?,
            ],
            started_by: [
                (methods >> 6).try_into()?,
                ((methods >> 4) & 3).try_into()?,
                ((methods >> 2) & 3).try_into()?,
            ],
            discharge_minutes,
            remaining_charge: level(charge),
        })
    }
    pub fn encode(self) -> Result<[u8; 6], CodecError> {
        if self.discharge_minutes > 510 {
            return Err(CodecError::OutsideRange);
        }
        let [f, d, p] = self.last;
        let [sf, sd, sp] = self.started_by;
        let [h, l] = self.discharge_minutes.to_be_bytes();
        Ok([
            (f as u8) << 4 | d as u8,
            (p as u8) << 4,
            (sf as u8) << 6 | (sd as u8) << 4 | (sp as u8) << 2,
            h,
            l,
            pack_level(self.remaining_charge)?,
        ])
    }
}

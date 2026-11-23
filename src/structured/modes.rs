use crate::{CodecError, codecs::bytes, domain::wire_enum};

wire_enum!("20.001", SystemClockMode {
    Autonomous = 0,
    Slave = 1,
    Master = 2,
});
wire_enum!("20.002", BuildingMode {
    InUse = 0,
    Unused = 1,
    Protection = 2,
});
wire_enum!("20.003", OccupancyMode {
    Occupied = 0,
    Standby = 1,
    Unoccupied = 2,
});
wire_enum!("20.004", ParameterPriority {
    High = 0,
    Medium = 1,
    Low = 2,
    Void = 3,
});
wire_enum!("20.006", ApplicationArea {
    NoFault = 0,
    System = 1,
    GeneralHvac = 10,
    HotWaterHeating = 11,
    DirectElectricalHeating = 12,
    TerminalUnits = 13,
    VentilationAirConditioning = 14,
    Lighting = 20,
    Security = 30,
    LoadManagement = 40,
    ShuttersBlinds = 50,
});
wire_enum!("20.007", AlarmClass {
    Simple = 1,
    Basic = 2,
    Extended = 3,
});
wire_enum!("20.008", PowerSupplyMode {
    Disabled = 0,
    Enabled = 1,
    Automatic = 2,
});
wire_enum!("20.011", SystemFault {
    NoFault = 0,
    Device = 1,
    Communication = 2,
    Configuration = 3,
    Hardware = 4,
    Software = 5,
    InsufficientNonvolatileMemory = 6,
    InsufficientVolatileMemory = 7,
    ZeroSizeAllocation = 8,
    Crc = 9,
    WatchdogReset = 10,
    InvalidOpcode = 11,
    GeneralProtection = 12,
    TableLengthExceeded = 13,
    UndefinedLoadCommand = 14,
    UnsortedGroupAddresses = 15,
    InvalidConnectionNumber = 16,
    InvalidGroupObjectNumber = 17,
    GroupObjectTypeTooLong = 18,
});
wire_enum!("20.012", HvacFault {
    NoFault = 0,
    Sensor = 1,
    ProcessOrController = 2,
    Actuator = 3,
    Other = 4,
});
wire_enum!("20.014", WindForce {
    Calm = 0,
    LightAir = 1,
    LightBreeze = 2,
    GentleBreeze = 3,
    ModerateBreeze = 4,
    FreshBreeze = 5,
    StrongBreeze = 6,
    NearGale = 7,
    FreshGale = 8,
    StrongGale = 9,
    Storm = 10,
    ViolentStorm = 11,
    Hurricane = 12,
});
wire_enum!("20.017", SensorSelection {
    Inactive = 0,
    Digital = 1,
    DigitalInverted = 2,
    AnalogPercent = 3,
    Temperature = 4,
});
wire_enum!("20.020", ActuatorConnection {
    Sensor = 1,
    Controller = 2,
});
wire_enum!("20.021", CloudCover {
    Cloudless = 0,
    Sunny = 1,
    Sunshiny = 2,
    LightlyCloudy = 3,
    ScatteredClouds = 4,
    Cloudy = 5,
    SixOktas = 6,
    SevenOktas = 7,
    Overcast = 8,
    Obstructed = 9,
});
wire_enum!("20.022", PowerReturnMode {
    DoNotSend = 0,
    AlwaysSend = 1,
    SendChanged = 2,
});
wire_enum!("23.001", SwitchAction {
    Off = 0,
    On = 1,
    OffThenOn = 2,
    OnThenOff = 3,
});
wire_enum!("23.002", AlarmReaction {
    Unused = 0,
    Up = 1,
    Down = 2,
});
wire_enum!("23.003", TravelAction {
    Up = 0,
    Down = 1,
    UpThenDown = 2,
    DownThenUp = 3,
});

/// DPT 20.005. Values 17 through 255 are manufacturer-specific.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LightApplicationMode {
    Normal,
    PresenceSimulation,
    NightRound,
    Manufacturer(u8),
}
impl LightApplicationMode {
    pub fn decode(input: &[u8]) -> Result<Self, CodecError> {
        match bytes(input)? {
            [0] => Ok(Self::Normal),
            [1] => Ok(Self::PresenceSimulation),
            [2] => Ok(Self::NightRound),
            [c @ 17..=255] => Ok(Self::Manufacturer(c)),
            _ => Err(CodecError::InvalidData),
        }
    }
    pub fn encode(self) -> Result<[u8; 1], CodecError> {
        Ok([match self {
            Self::Normal => 0,
            Self::PresenceSimulation => 1,
            Self::NightRound => 2,
            Self::Manufacturer(c @ 17..=255) => c,
            _ => return Err(CodecError::OutsideRange),
        }])
    }
}
const DELAYS: [u32; 26] = [
    0, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 75, 90, 120, 150, 180, 300, 900, 1200, 1800, 3600, 7200,
    10800, 18000, 43200, 86400,
];
/// DPT 20.013 parameter delay in seconds. Zero disables the delay.
pub fn decode_time_delay_seconds(input: &[u8]) -> Result<u32, CodecError> {
    let [code] = bytes(input)?;
    DELAYS
        .get(usize::from(code))
        .copied()
        .ok_or(CodecError::InvalidData)
}
pub fn encode_time_delay_seconds(seconds: u32) -> Result<[u8; 1], CodecError> {
    DELAYS
        .iter()
        .position(|v| *v == seconds)
        .map(|i| [i as u8])
        .ok_or(CodecError::OutsideRange)
}

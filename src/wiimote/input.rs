use crate::prelude::*;
use bitflags::bitflags;

const STATUS_ID: u8 = 0x20;
const READ_MEMORY_ID: u8 = 0x21;
const ACKNOWLEDGE_ID: u8 = 0x22;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct StatusFlags: u8 {
        const BATTERY_LOW = 0b0000_0001;
        const EXTENSION_CONTROLLER_CONNECTED = 0b0000_0010;
        const SPEAKER_ENABLED = 0b0000_0100;
        const IR_CAMERA_ENABLED = 0b0000_1000;
        const LED_1 = 0b0001_0000;
        const LED_2 = 0b0010_0000;
        const LED_3 = 0b0100_0000;
        const LED_4 = 0b1000_0000;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonData {
    pub first: u8,
    pub second: u8,
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct StatusData {
    pub buttons: ButtonData,
    pub flags: StatusFlags,
    _reserved: [u8; 2],
    pub battery_level: u8,
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct MemoryData {
    pub buttons: ButtonData,
    size_error_flags: u8,
    address: [u8; 2],
    pub data: [u8; 16],
}

impl MemoryData {
    #[must_use]
    pub const fn size(&self) -> u8 {
        (self.size_error_flags >> 4) + 1
    }

    #[must_use]
    pub const fn error_flag(&self) -> u8 {
        self.size_error_flags & 0x0F
    }

    #[must_use]
    pub const fn address_offset(&self) -> u16 {
        u16::from_be_bytes(self.address)
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct AcknowledgeData {
    pub buttons: ButtonData,
    pub report_number: u8,
    pub error_code: u8,
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct WiimoteData {
    pub data: [u8; 21],
}

#[derive(Debug)]
pub enum InputReport {
    StatusInformation(StatusData),
    ReadMemory(MemoryData),
    Acknowledge(AcknowledgeData),
    DataReport(u8, WiimoteData),
}

macro_rules! transmute_data {
    ($value:expr, $type:ident) => {{
        const DATA_SIZE: usize = std::mem::size_of::<$type>();
        let mut slice = [0u8; DATA_SIZE];
        slice.copy_from_slice(&$value[1..=DATA_SIZE]);

        unsafe { std::mem::transmute::<[u8; DATA_SIZE], $type>(slice) }
    }};
}

impl InputReport {
    fn from_status_information(value: [u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]) -> Self {
        let data = transmute_data!(value, StatusData);
        Self::StatusInformation(data)
    }

    fn from_read_memory_data(value: [u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]) -> Self {
        let data = transmute_data!(value, MemoryData);
        Self::ReadMemory(data)
    }

    fn from_acknowledge(value: [u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]) -> Self {
        let data = transmute_data!(value, AcknowledgeData);
        Self::Acknowledge(data)
    }

    fn from_data_report(value: [u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]) -> Self {
        let data = transmute_data!(value, WiimoteData);
        Self::DataReport(value[0], data)
    }
}

impl TryFrom<[u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]> for InputReport {
    type Error = WiimoteError;

    fn try_from(value: [u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]) -> Result<Self, Self::Error> {
        match value[0] {
            STATUS_ID => Ok(Self::from_status_information(value)),
            READ_MEMORY_ID => Ok(Self::from_read_memory_data(value)),
            ACKNOWLEDGE_ID => Ok(Self::from_acknowledge(value)),
            0x30..=0x3F => Ok(Self::from_data_report(value)),
            _ => Err(WiimoteDeviceError::InvalidData.into()),
        }
    }
}

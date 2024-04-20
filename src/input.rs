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

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct ButtonData: u16 {
        const LEFT = 1 << 0;
        const RIGHT = 1 << 1;
        const DOWN = 1 << 2;
        const UP = 1 << 3;
        const PLUS = 1 << 4;

        const TWO = 1 << 8;
        const ONE = 1 << 9;
        const B = 1 << 10;
        const A = 1 << 11;
        const MINUS = 1 << 12;

        const HOME = 1 << 15;
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct StatusData {
    buttons: ButtonData,
    flags: StatusFlags,
    _reserved: [u8; 2],
    battery_level: u8,
}

impl StatusData {
    /// Returns the core button data.
    #[must_use]
    pub const fn buttons(&self) -> ButtonData {
        self.buttons
    }

    /// Returns the status flags.
    #[must_use]
    pub const fn flags(&self) -> StatusFlags {
        self.flags
    }

    /// Returns the battery level.
    #[must_use]
    pub const fn battery_level(&self) -> u8 {
        self.battery_level
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct MemoryData {
    buttons: ButtonData,
    size_error_flags: u8,
    address: [u8; 2],
    pub data: [u8; 16],
}

impl MemoryData {
    /// Returns the core button data.
    #[must_use]
    pub const fn buttons(&self) -> ButtonData {
        self.buttons
    }

    /// Returns the size of the data in bytes.
    #[must_use]
    pub const fn size(&self) -> u8 {
        (self.size_error_flags >> 4) + 1
    }

    /// Returns the error flag.
    ///
    /// Known values:
    /// - 0: No error
    /// - 7: Attempted to read from write-only register or disconnected extension
    /// - 8: Attempted to read from non-existing address
    #[must_use]
    pub const fn error_flag(&self) -> u8 {
        self.size_error_flags & 0x0F
    }

    /// Returns the 2 least significant bytes of the address of the first byte.
    #[must_use]
    pub const fn address_offset(&self) -> u16 {
        u16::from_be_bytes(self.address)
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct AcknowledgeData {
    buttons: ButtonData,
    report_number: u8,
    error_code: u8,
}

impl AcknowledgeData {
    /// Returns the core button data.
    #[must_use]
    pub const fn buttons(&self) -> ButtonData {
        self.buttons
    }

    /// Returns the report number.
    #[must_use]
    pub const fn report_number(&self) -> u8 {
        self.report_number
    }

    /// Returns the error code.
    #[must_use]
    pub const fn error_code(&self) -> u8 {
        self.error_code
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct WiimoteData {
    pub data: [u8; 21],
}

impl WiimoteData {
    /// Returns the core button data.
    ///
    /// This is invalid for report type 0x3d that only contains extension data.
    #[must_use]
    pub const fn buttons(&self) -> ButtonData {
        let bits = u16::from_le_bytes([self.data[0], self.data[1]]);
        ButtonData::from_bits_retain(bits)
    }
}

/// An input report represents the data sent from the Wii remote to the computer.
#[derive(Debug)]
pub enum InputReport {
    /// Status information report (ID 0x20).
    ///
    /// Can be requested by sending an output report with ID 0x15 and is automatically
    /// sent when the Extension is connected or disconnected.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#0x20:_Status
    StatusInformation(StatusData),
    /// Read memory data report (ID 0x21).
    ///
    /// Result of a read memory request (output report ID 0x17).
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#0x21:_Read_Memory_Data
    ReadMemory(MemoryData),
    /// Acknowledge report (ID 0x22).
    ///
    /// Sent as a response to an output report with a corresponding result or error.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#0x22:_Acknowledge_output_report.2C_return_function_result
    Acknowledge(AcknowledgeData),
    /// Data report (IDs 0x30-0x3F).
    ///
    /// Contains the data of the buttons, accelerometer, IR and Extension from the Wii remote.
    /// The exact data depends on the report type requested by the output report 0x12.
    /// Defaults to 0x30 which only contains the button data.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Data_Reporting
    DataReport(u8, WiimoteData),
}

macro_rules! transmute_data {
    ($value:expr, $type:ident) => {{
        const DATA_SIZE: usize = std::mem::size_of::<$type>();
        if $value.len() < DATA_SIZE {
            return Err(WiimoteDeviceError::InvalidData.into());
        }
        let mut slice = [0u8; DATA_SIZE];
        slice.copy_from_slice(&$value[1..=DATA_SIZE]);

        unsafe { std::mem::transmute::<[u8; DATA_SIZE], $type>(slice) }
    }};
}

impl InputReport {
    fn from_status_information(value: &[u8]) -> WiimoteResult<Self> {
        let data = transmute_data!(value, StatusData);
        Ok(Self::StatusInformation(data))
    }

    fn from_read_memory_data(value: &[u8]) -> WiimoteResult<Self> {
        let data = transmute_data!(value, MemoryData);
        Ok(Self::ReadMemory(data))
    }

    fn from_acknowledge(value: &[u8]) -> WiimoteResult<Self> {
        let data = transmute_data!(value, AcknowledgeData);
        Ok(Self::Acknowledge(data))
    }

    fn from_data_report(value: &[u8]) -> Self {
        const DATA_SIZE: usize = 21;
        let mut data = [0u8; DATA_SIZE];
        let bytes_to_copy = usize::min(value.len() - 1, DATA_SIZE);
        data[..bytes_to_copy].copy_from_slice(&value[1..=bytes_to_copy]);

        Self::DataReport(value[0], WiimoteData { data })
    }
}

impl TryFrom<&[u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]> for InputReport {
    type Error = WiimoteError;

    fn try_from(value: &[u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE]) -> Result<Self, Self::Error> {
        let slice_without_length: &[u8] = value.as_slice();
        Self::try_from(slice_without_length)
    }
}

impl TryFrom<&[u8]> for InputReport {
    type Error = WiimoteError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(WiimoteDeviceError::MissingData.into());
        }
        match value[0] {
            STATUS_ID => Self::from_status_information(value),
            READ_MEMORY_ID => Self::from_read_memory_data(value),
            ACKNOWLEDGE_ID => Self::from_acknowledge(value),
            0x30..=0x3F => Ok(Self::from_data_report(value)),
            _ => Err(WiimoteDeviceError::InvalidData.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_report() {
        let mut data = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];
        data[0] = 0x20;
        data[1] = 0b0001_0100; // Plus and D-Pad down
        data[2] = 0b0000_0100; // B
        data[3] = 0b0010_0101; // Status (battery low, speaker, led 2)

        data[6] = 24; // Battery level

        let report = InputReport::try_from(&data).unwrap();

        assert!(matches!(report, InputReport::StatusInformation(_)));
        if let InputReport::StatusInformation(data) = report {
            assert_eq!(
                data.buttons().bits(),
                ButtonData::DOWN
                    .union(ButtonData::PLUS)
                    .union(ButtonData::B)
                    .bits()
            );
            assert_eq!(
                data.flags().bits(),
                StatusFlags::BATTERY_LOW
                    .union(StatusFlags::SPEAKER_ENABLED)
                    .union(StatusFlags::LED_2)
                    .bits()
            );
            assert_eq!(data.battery_level(), 24);
        }
    }

    #[test]
    fn test_read_memory_report() {
        let mut data = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];
        data[0] = 0x21;
        data[1] = 0b0000_0000; // no button
        data[2] = 0b1000_0000; // Home
        data[3] = 0xF7; // Size and error flags
        data[4] = 0x12; // Address
        data[5] = 0xAB; // Address
        data[6..22].copy_from_slice(b"1234567890123456"); // Data

        let report = InputReport::try_from(&data).unwrap();

        assert!(matches!(report, InputReport::ReadMemory(_)));
        if let InputReport::ReadMemory(data) = report {
            assert_eq!(data.buttons().bits(), ButtonData::HOME.bits());
            assert_eq!(data.size(), 16);
            assert_eq!(data.error_flag(), 7);
            assert_eq!(data.address_offset(), 0x12AB);
            assert_eq!(data.data, *b"1234567890123456");
        }
    }

    #[test]
    fn test_acknowledge_report() {
        let data: &[u8] = &[
            0x22,
            0b0000_0000, // no button
            0b0000_0000, // no button
            0x12,        // report number
            0xAB,        // error code
        ];

        let report = InputReport::try_from(data).unwrap();

        assert!(matches!(report, InputReport::Acknowledge(_)));
        if let InputReport::Acknowledge(data) = report {
            assert_eq!(data.buttons().bits(), 0);
            assert_eq!(data.report_number(), 0x12);
            assert_eq!(data.error_code(), 0xAB);
        }
    }

    #[test]
    fn test_buttons_mode_0x30() {
        let data: &[u8] = &[
            0x30,
            0b0000_0001, // D-Pad left
            0b0000_0010, // One
        ];

        let report = InputReport::try_from(data).unwrap();

        assert!(matches!(report, InputReport::DataReport(0x30, _)));
        if let InputReport::DataReport(_, data) = report {
            assert_eq!(
                data.buttons().bits(),
                ButtonData::LEFT.union(ButtonData::ONE).bits()
            );
        }
    }
}

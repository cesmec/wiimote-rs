use crate::prelude::*;
use bitflags::bitflags;

const RUMBLE_ID: u8 = 0x10;
const PLAYER_LED_ID: u8 = 0x11;
const DATA_REPORTING_MODE_ID: u8 = 0x12;
const IR_CAMERA_ENABLE_ID: u8 = 0x13;
const SPEAKER_ENABLE_ID: u8 = 0x14;
const STATUS_REQUEST_ID: u8 = 0x15;
const WRITE_MEMORY_ID: u8 = 0x16;
const READ_MEMORY_ID: u8 = 0x17;
const SPEAKER_DATA_ID: u8 = 0x18;
const SPEAKER_MUTE_ID: u8 = 0x19;
const IR_CAMERA_ENABLE_2_ID: u8 = 0x1A;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PlayerLedFlags: u8 {
        const LED_1 = 0b0001_0000;
        const LED_2 = 0b0010_0000;
        const LED_3 = 0b0100_0000;
        const LED_4 = 0b1000_0000;
    }
}

#[derive(Debug)]
pub struct DataReporingMode {
    pub continuous: bool,
    pub mode: u8,
}

#[derive(Debug)]
pub struct Addressing {
    /// If true, read from control registers, otherwise from EEPROM.
    control_registers: bool,
    pub(crate) address: u32,
    pub(crate) size: u16,
}

impl Addressing {
    #[must_use]
    pub const fn control_registers(address: u32, size: u16) -> Self {
        Self {
            control_registers: true,
            address,
            size,
        }
    }

    #[must_use]
    pub const fn eeprom(address: u32, size: u16) -> Self {
        Self {
            control_registers: false,
            address,
            size,
        }
    }
}

/// An output report represents the data sent from the computer to the Wii remote.
///
/// The least significant bit of the first byte of any output report enables or disables the rumble.
#[derive(Debug)]
pub enum OutputReport {
    /// Turn rumble on or off without any other changes.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Rumble
    Rumble(bool),
    /// Set the player LED lights.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Player_LEDs
    PlayerLed(PlayerLedFlags),
    /// Set the data reporting mode of the input reports.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Data_Reporting
    DataReportingMode(DataReporingMode),
    /// Enable or disable the IR camera (first step of enable sequence).
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#IR_Camera
    IrCameraEnable(bool),
    /// Enable or disable the built-in speaker.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Speaker
    SpeakerEnable(bool),
    /// Request a status input report from the Wii remote.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#0x20:_Status
    StatusRequest,
    /// Write up to 16 bytes of data to the Wii remote's memory or registers.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Memory_and_Registers
    WriteMemory(Addressing, [u8; 16]),
    /// Read data from the Wii remote's memory or registers.
    /// The data is returned as `InputReport::ReadMemory` reports in chunks of 16 bytes.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Memory_and_Registers
    ReadMemory(Addressing),
    /// Send data to the built-in speaker.
    /// The first byte is the length of the data, followed by the actual data.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Speaker
    SpeakerData(u8, [u8; 20]),
    /// Mute or unmute the built-in speaker.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#Speaker
    SpeakerMute(bool),
    /// Second step of IR camera enable sequence.
    ///
    /// WiiBrew Documentation: https://www.wiibrew.org/wiki/Wiimote#IR_Camera
    IrCameraEnable2(bool),
}

impl OutputReport {
    /// Converts the output report to a byte array.
    /// The rumble flag is used in all output reports to enable or disable the rumble motor.
    ///
    /// Returns a tuple containing the byte array and the actual length of the data.
    #[must_use]
    pub fn to_array(&self, rumble: bool) -> ([u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE], usize) {
        let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];
        let length = self.fill_buffer(rumble, &mut buffer);
        (buffer, length)
    }

    /// Fills an existing buffer with the output report data.
    /// The rumble flag is used in all output reports to enable or disable the rumble motor.
    ///
    /// Returns the actual length of the data.
    pub fn fill_buffer(&self, mut rumble: bool, buffer: &mut [u8]) -> usize {
        buffer[1] = 0;
        let length = match self {
            Self::Rumble(rumble_enabled) => {
                rumble = *rumble_enabled;
                buffer[0] = RUMBLE_ID;
                2
            }
            Self::PlayerLed(flags) => {
                buffer[0] = PLAYER_LED_ID;
                buffer[1] = flags.bits();
                2
            }
            Self::DataReportingMode(mode) => {
                buffer[0] = DATA_REPORTING_MODE_ID;
                buffer[1] = if mode.continuous { 0x04 } else { 0x00 };
                buffer[2] = mode.mode;
                3
            }
            Self::IrCameraEnable(enable) => {
                buffer[0] = IR_CAMERA_ENABLE_ID;
                buffer[1] = if *enable { 0x04 } else { 0x00 };
                2
            }
            Self::SpeakerEnable(enable) => {
                buffer[0] = SPEAKER_ENABLE_ID;
                buffer[1] = if *enable { 0x04 } else { 0x00 };
                2
            }
            Self::StatusRequest => {
                buffer[0] = STATUS_REQUEST_ID;
                2
            }
            Self::WriteMemory(addressing, data) => {
                buffer[0] = WRITE_MEMORY_ID;
                buffer[1..=4].copy_from_slice(&addressing.address.to_be_bytes());
                // Address is 3 bytes long, byte 1 is used for control register and rumble.
                buffer[1] = if addressing.control_registers {
                    0x04
                } else {
                    0x00
                };
                buffer[5] = u8::min(addressing.size as u8, 16);
                buffer[6..=21].copy_from_slice(data);
                22
            }
            Self::ReadMemory(addressing) => {
                buffer[0] = READ_MEMORY_ID;
                buffer[1..=4].copy_from_slice(&addressing.address.to_be_bytes());
                // Address is 3 bytes long, byte 1 is used for control register and rumble.
                buffer[1] = if addressing.control_registers {
                    0x04
                } else {
                    0x00
                };
                buffer[5..=6].copy_from_slice(&addressing.size.to_be_bytes());
                7
            }
            Self::SpeakerData(length, data) => {
                buffer[0] = SPEAKER_DATA_ID;
                buffer[1] = (*length) << 3;
                buffer[2..=21].copy_from_slice(data);
                22
            }
            Self::SpeakerMute(mute) => {
                buffer[0] = SPEAKER_MUTE_ID;
                buffer[1] = if *mute { 0x04 } else { 0x00 };
                2
            }
            Self::IrCameraEnable2(enable) => {
                buffer[0] = IR_CAMERA_ENABLE_2_ID;
                buffer[1] = if *enable { 0x04 } else { 0x00 };
                2
            }
        };
        if rumble {
            // https://www.wiibrew.org/wiki/Wiimote#Rumble
            // ... the rumble motor can be turned on or off through any of the Output Reports, not just 0x10.
            // Setting the LSB (bit 0) of the first byte of any output report will activate the rumble motor,
            // and unsetting it will deactivate it.
            buffer[1] |= 0x01;
        }
        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rumble_report() {
        let report = OutputReport::Rumble(true);

        let (buffer, size) = report.to_array(false); // value here is overwritten by report

        assert_eq!(size, 2);
        assert_eq!(buffer[0], RUMBLE_ID);
        assert_eq!(buffer[1], 0b0000_0001);
    }

    #[test]
    fn test_rumble_in_player_led_report() {
        let report = OutputReport::PlayerLed(PlayerLedFlags::LED_2);

        let (buffer, size) = report.to_array(true);

        assert_eq!(size, 2);
        assert_eq!(buffer[0], PLAYER_LED_ID);
        assert_eq!(buffer[1], 0b0010_0001);
    }

    #[test]
    fn test_player_led_report() {
        let report = OutputReport::PlayerLed(PlayerLedFlags::LED_1 | PlayerLedFlags::LED_3);

        let (buffer, size) = report.to_array(false);

        assert_eq!(size, 2);
        assert_eq!(buffer[0], PLAYER_LED_ID);
        assert_eq!(buffer[1], 0b0101_0000);
    }

    #[test]
    fn test_read_report() {
        let addressing = Addressing::control_registers(0xFF12_3456, 8);
        let report = OutputReport::ReadMemory(addressing);

        let (buffer, size) = report.to_array(true);

        assert_eq!(size, 7);
        assert_eq!(buffer[0], READ_MEMORY_ID);
        assert_eq!(buffer[1], 0x04 | 0b0000_0001); // control register and rumble
        assert_eq!(buffer[2], 0x12);
        assert_eq!(buffer[3], 0x34);
        assert_eq!(buffer[4], 0x56);
        assert_eq!(buffer[5], 0);
        assert_eq!(buffer[6], 8);
    }

    #[test]
    fn test_write_report() {
        let addressing = Addressing::eeprom(0x89AB_CD00, 11);
        let report = OutputReport::WriteMemory(addressing, *b"12345678901\0\0\0\0\0");

        let (buffer, size) = report.to_array(false);

        assert_eq!(size, 22);
        assert_eq!(buffer[0], WRITE_MEMORY_ID);
        assert_eq!(buffer[1], 0); // eeprom and no rumble
        assert_eq!(buffer[2], 0xAB);
        assert_eq!(buffer[3], 0xCD);
        assert_eq!(buffer[4], 0x00);
        assert_eq!(buffer[5], 11);
        assert_eq!(&buffer[6..=21], *b"12345678901\0\0\0\0\0");
    }

    #[test]
    fn test_speaker_data_report() {
        let report = OutputReport::SpeakerData(20, *b"12345678901234567890");

        let (buffer, size) = report.to_array(true);

        assert_eq!(size, 22);
        assert_eq!(buffer[0], SPEAKER_DATA_ID);
        assert_eq!(buffer[1], (20 << 3) | 1); // length and rumble
        assert_eq!(&buffer[2..=21], *b"12345678901234567890");
    }
}

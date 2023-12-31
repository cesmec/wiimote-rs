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

#[derive(Debug)]
pub enum OutputReport {
    Rumble,
    PlayerLed(PlayerLedFlags),
    DataReportingMode(DataReporingMode),
    IrCameraEnable(bool),
    SpeakerEnable(bool),
    StatusRequest,
    WriteMemory(Addressing, [u8; 16]),
    ReadMemory(Addressing),
    SpeakerData(u8, [u8; 20]),
    SpeakerMute(bool),
    IrCameraEnable2(bool),
}

impl OutputReport {
    #[must_use]
    pub fn to_array(&self, rumble: bool) -> ([u8; WIIMOTE_REPORT_BUFFER_SIZE], usize) {
        let mut buffer = [0u8; WIIMOTE_REPORT_BUFFER_SIZE];
        let length = self.fill_buffer(rumble, &mut buffer);
        (buffer, length)
    }

    pub fn fill_buffer(&self, rumble: bool, buffer: &mut [u8]) -> usize {
        buffer[1] = 0;
        let length = match self {
            Self::Rumble => {
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

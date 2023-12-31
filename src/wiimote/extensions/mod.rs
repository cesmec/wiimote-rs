mod motion_plus;

use crate::prelude::*;
use crate::wiimote::simple_io;

pub use motion_plus::{MotionPlus, MotionPlusCalibration, MotionPlusMode};

#[derive(Debug)]
pub enum WiimoteExtension {
    Nunchuck,
    ClassicController,
    ClassicControllerPro,
    BalanceBoard,
    Unknown([u8; 6]),
}

impl WiimoteExtension {
    /// Detects the extension (except for Motion Plus) connected to the Wii remote.
    ///
    /// # Errors
    ///
    /// This function will return an error on I/O error or if invalid data is received.
    pub fn detect(wiimote: &WiimoteDevice) -> WiimoteResult<Option<Self>> {
        let identifier = Self::identify_extension(wiimote)?;

        // https://www.wiibrew.org/wiki/Wiimote/Extension_Controllers#Identification
        Ok(match identifier {
            Some([_, _, 0xA4, 0x20, 0x00, 0x00]) => Some(Self::Nunchuck),
            Some([0x01, 0x00, 0xA4, 0x20, 0x01, 0x01]) => Some(Self::ClassicControllerPro),
            Some([_, _, 0xA4, 0x20, 0x01, 0x01]) => Some(Self::ClassicController),
            Some([_, _, 0xA4, 0x20, 0x04, 0x02]) => Some(Self::BalanceBoard),
            Some(identifier) => Some(Self::Unknown(identifier)),
            None => None,
        })
    }

    fn identify_extension(wiimote: &WiimoteDevice) -> WiimoteResult<Option<[u8; 6]>> {
        // https://www.wiibrew.org/wiki/Wiimote/Extension_Controllers#Identification
        // The new way to initialize the extension is by writing 0x55 to 0x(4)A400F0, then writing 0x00 to 0x(4)A400FB.
        // Once initialized, the last six bytes of the register block identify the connected Extension Controller.
        // A six-byte read of register 0xA400FA will return these bytes.
        // The Extension Controller must have been initialized prior to this.
        let mut memory_write_buffer = [0u8; 16];

        memory_write_buffer[0] = 0x55;
        let addressing = Addressing::control_registers(0xA4_00F0, 1);
        let ack = simple_io::write_16_bytes_sync(wiimote, addressing, &memory_write_buffer)?;
        if ack.error_code == 7 {
            return Ok(None);
        }

        memory_write_buffer[0] = 0x00;
        let addressing = Addressing::control_registers(0xA4_00FB, 1);
        let ack = simple_io::write_16_bytes_sync(wiimote, addressing, &memory_write_buffer)?;
        if ack.error_code == 7 {
            return Ok(None);
        }

        let addressing = Addressing::control_registers(0xA4_00FA, 6);
        let read_result = simple_io::read_16_bytes_sync(wiimote, addressing)?;
        // Address is actually 0xA4_00FA, but only the lower 2 bytes are returned
        if read_result.address_offset() != 0x00FA || read_result.size() < 6 {
            Err(WiimoteDeviceError::InvalidData.into())
        } else if read_result.error_flag() == 7 {
            Ok(None)
        } else {
            let mut extension_info = [0u8; 6];
            extension_info.copy_from_slice(&read_result.data[..6]);
            Ok(Some(extension_info))
        }
    }
}

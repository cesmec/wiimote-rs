use std::cell::RefCell;
use std::sync::atomic::AtomicBool;

use crate::prelude::*;
use crate::wiimote::simple_io;

#[derive(Debug, Clone, Copy)]
pub enum MotionPlusMode {
    Inactive,
    Active,
    NunchuckPassthrough,
    ClassicControllerPassthrough,
}

#[derive(Debug, Clone, Copy)]
pub enum MotionPlusType {
    External,
    Builtin,
}

#[derive(Debug, Default, Clone)]
pub struct MotionPlusCalibration {
    fast: MotionPlusCalibrationData,
    slow: MotionPlusCalibrationData,
}

#[derive(Debug, Default, Clone)]
struct MotionPlusCalibrationData {
    yaw_zero_value: u16,
    roll_zero_value: u16,
    pitch_zero_value: u16,
    yaw_scale: u16,
    roll_scale: u16,
    pitch_scale: u16,
    degrees_div_6: u8,
}

impl From<[u8; 16]> for MotionPlusCalibrationData {
    fn from(value: [u8; 16]) -> Self {
        Self {
            yaw_zero_value: u16::from_be_bytes([value[0], value[1]]),
            roll_zero_value: u16::from_be_bytes([value[2], value[3]]),
            pitch_zero_value: u16::from_be_bytes([value[4], value[5]]),
            yaw_scale: u16::from_be_bytes([value[6], value[7]]),
            roll_scale: u16::from_be_bytes([value[8], value[9]]),
            pitch_scale: u16::from_be_bytes([value[10], value[11]]),
            degrees_div_6: value[12],
        }
    }
}

#[derive(Debug)]
pub struct MotionPlus {
    motion_plus_type: MotionPlusType,
    initialized: AtomicBool,
    mode: RefCell<MotionPlusMode>,
    calibration: RefCell<MotionPlusCalibration>,
}

// https://www.wiibrew.org/wiki/Wiimote/Extension_Controllers/Wii_Motion_Plus
impl MotionPlus {
    /// Detects if the Wii remote has a Motion Plus extension.
    ///
    /// # Errors
    ///
    /// This function will return an error if communication to the Wii remote failed.
    pub(crate) fn detect(wiimote: &WiimoteDevice) -> WiimoteResult<Option<Self>> {
        let address = Addressing::control_registers(0xA6_00FA, 6);
        let memory_data = simple_io::read_16_bytes_sync(wiimote, address)?;
        let motion_plus_type = match memory_data.data[0..6] {
            [0x00, 0x00, 0xA6, 0x20, _, 0x05] => MotionPlusType::External,
            [_, 0x00, 0xA6, 0x20, _, 0x05] => MotionPlusType::Builtin,
            _ => return Ok(None),
        };

        Ok(Some(Self {
            motion_plus_type,
            initialized: AtomicBool::new(false),
            mode: RefCell::new(MotionPlusMode::Inactive),
            calibration: RefCell::new(MotionPlusCalibration::default()),
        }))
    }

    #[must_use]
    pub const fn motion_plus_type(&self) -> MotionPlusType {
        self.motion_plus_type
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::Relaxed)
    }

    #[must_use]
    pub fn mode(&self) -> MotionPlusMode {
        *self.mode.borrow()
    }

    #[must_use]
    pub fn calibration(&self) -> MotionPlusCalibration {
        self.calibration.borrow().clone()
    }

    /// Tries to initialize the Motion Plus extension and read its calibration.
    ///
    /// # Errors
    ///
    /// This function will return an error on I/O error or when receiving invalid data.
    pub fn initialize(&self, wiimote: &WiimoteDevice) -> WiimoteResult<()> {
        Self::write_single_control_byte(wiimote, 0xA6_00F0, 0x55)?;
        self.read_calibration_data(wiimote)?;
        self.initialized
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Changes the mode of the Motion Plus extension.
    ///
    /// # Errors
    ///
    /// This function will return an error on I/O error or when receiving invalid data.
    pub fn change_mode(&self, wiimote: &WiimoteDevice, mode: MotionPlusMode) -> WiimoteResult<()> {
        let (address, value) = match mode {
            MotionPlusMode::Inactive => (0xA4_00F0, 0x55),
            MotionPlusMode::Active => (0xA6_00FE, 0x04),
            MotionPlusMode::NunchuckPassthrough => (0xA6_00FE, 0x05),
            MotionPlusMode::ClassicControllerPassthrough => (0xA6_00FE, 0x07),
        };
        Self::write_single_control_byte(wiimote, address, value)?;
        self.mode.replace(mode);
        Ok(())
    }

    fn write_single_control_byte(
        wiimote: &WiimoteDevice,
        address: u32,
        value: u8,
    ) -> WiimoteResult<()> {
        let addressing = Addressing::control_registers(address, 1);
        let mut memory_write_buffer = [0u8; 16];
        memory_write_buffer[0] = value;
        let ack = simple_io::write_16_bytes_sync(wiimote, addressing, &memory_write_buffer)?;
        if ack.error_code == 7 {
            return Err(WiimoteDeviceError::InvalidData.into());
        }

        Ok(())
    }

    fn read_calibration_data(&self, wiimote: &WiimoteDevice) -> WiimoteResult<()> {
        let mut hasher = crc32fast::Hasher::new();
        let mut checksum = [0u8; 4];

        let fast =
            Self::read_calibration_part(wiimote, 0xA6_0020, &mut hasher, &mut checksum[0..2])?;
        let slow =
            Self::read_calibration_part(wiimote, 0xA6_0020 + 16, &mut hasher, &mut checksum[2..4])?;

        if hasher.finalize() != u32::from_be_bytes(checksum) {
            return Err(WiimoteDeviceError::InvalidChecksum.into());
        }

        self.calibration
            .replace(MotionPlusCalibration { fast, slow });
        Ok(())
    }

    fn read_calibration_part(
        wiimote: &WiimoteDevice,
        address: u32,
        hasher: &mut crc32fast::Hasher,
        checksum_buffer: &mut [u8],
    ) -> WiimoteResult<MotionPlusCalibrationData> {
        let addressing = Addressing::control_registers(address, 16);
        let data = simple_io::read_16_bytes_sync_checked(wiimote, addressing)?;
        hasher.update(&data[0..14]);
        checksum_buffer.copy_from_slice(&data[14..16]);
        Ok(MotionPlusCalibrationData::from(data))
    }
}

use std::cell::RefCell;
use std::sync::atomic::AtomicBool;

use crate::calibration::normalize;
use crate::output::Addressing;
use crate::prelude::*;
use crate::simple_io;

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

impl MotionPlusCalibration {
    #[must_use]
    pub fn get_angular_velocity(&self, data: &MotionPlusData) -> (f64, f64, f64) {
        // https://www.wiibrew.org/wiki/Wiimote/Extension_Controllers/Wii_Motion_Plus#Data_Format
        const UNIT_PER_DEG_PER_S: f64 = 8192.0 / 595.0;
        const HIGH_SPEED_MULTIPLIER: f64 = 2000.0 / 440.0;

        #[rustfmt::skip]
        let calibration = (
            if data.yaw_slow { &self.slow } else { &self.fast },
            if data.roll_slow { &self.slow } else { &self.fast },
            if data.pitch_slow { &self.slow } else { &self.fast },
        );

        // At high speed (slow bit = 0) raw values read are small with the same deg/s to reach
        // higher values on top, so you must multiply it by 2000/440
        #[rustfmt::skip]
        let mode_multiplier = (
            if data.yaw_slow { 1.0 } else { HIGH_SPEED_MULTIPLIER },
            if data.roll_slow { 1.0 } else { HIGH_SPEED_MULTIPLIER },
            if data.pitch_slow { 1.0 } else { HIGH_SPEED_MULTIPLIER },
        );

        let scale = (
            calibration.0.yaw_scale,
            calibration.1.roll_scale,
            calibration.2.pitch_scale,
        );
        let zero = (
            calibration.0.yaw_zero_value,
            calibration.1.roll_zero_value,
            calibration.2.pitch_zero_value,
        );
        let degrees = (
            calibration.0.degrees_div_6 as f64 * 6_f64,
            calibration.1.degrees_div_6 as f64 * 6_f64,
            calibration.2.degrees_div_6 as f64 * 6_f64,
        );

        let yaw: f64 = normalize(data.yaw, 14, zero.0, scale.0, 16);
        let roll: f64 = normalize(data.roll, 14, zero.1, scale.1, 16);
        let pitch: f64 = normalize(data.pitch, 14, zero.2, scale.2, 16);

        (
            yaw * degrees.0 * mode_multiplier.0 / UNIT_PER_DEG_PER_S,
            roll * degrees.1 * mode_multiplier.1 / UNIT_PER_DEG_PER_S,
            pitch * degrees.2 * mode_multiplier.2 / UNIT_PER_DEG_PER_S,
        )
    }
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

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct MotionPlusData {
    pub yaw: u16,
    pub roll: u16,
    pub pitch: u16,
    pub yaw_slow: bool,
    pub roll_slow: bool,
    pub pitch_slow: bool,
    pub extension_connected: bool,
}

impl TryFrom<[u8; 6]> for MotionPlusData {
    type Error = ();

    fn try_from(value: [u8; 6]) -> Result<Self, Self::Error> {
        // https://www.wiibrew.org/wiki/Wiimote/Extension_Controllers/Wii_Motion_Plus#Nunchuck_pass-through_mode
        // Bit 1 of Byte 5 is used to determine which type of report is received:
        // it is 1 when it contains MotionPlus Data and 0 when it contains extension data.
        let is_motion_plus_data = value[5] & 0b10 == 0b10;
        if !is_motion_plus_data {
            return Err(());
        }

        Ok(Self {
            yaw: u16::from_be_bytes([value[3] >> 2, value[0]]),
            roll: u16::from_be_bytes([value[4] >> 2, value[1]]),
            pitch: u16::from_be_bytes([value[5] >> 2, value[2]]),
            yaw_slow: value[3] & 0b0010 != 0,
            roll_slow: value[3] & 0b0001 != 0,
            pitch_slow: value[4] & 0b0010 != 0,
            extension_connected: value[4] & 0b0001 != 0,
        })
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

    /// Calibrates the slow zero values of the Motion Plus extension using multiple data readings.
    /// Cancels calibration if too much movement is detected (any of the slow flags set to false).
    /// Returns the new calibration data if successful.
    pub fn calibrate_zero_values(
        &self,
        readings: &[MotionPlusData],
    ) -> Option<MotionPlusCalibration> {
        let read_count = readings.len();
        let mut data = (0, 0, 0);

        for reading in readings {
            if !reading.yaw_slow || !reading.roll_slow || !reading.pitch_slow {
                // Too much movement, abort calibration
                return None;
            }

            data.0 += reading.yaw as u64;
            data.1 += reading.roll as u64;
            data.2 += reading.pitch as u64;
        }

        #[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
        if read_count >= 8 {
            let average_yaw = ((data.0 as f64 / read_count as f64).round() as u16) << 2; // Calibration has 16 bits, values only 14
            let average_roll = ((data.1 as f64 / read_count as f64).round() as u16) << 2;
            let average_pitch = ((data.2 as f64 / read_count as f64).round() as u16) << 2;

            let mut calibration = self.calibration.borrow_mut();

            calibration.slow.yaw_zero_value = average_yaw;
            calibration.slow.roll_zero_value = average_roll;
            calibration.slow.pitch_zero_value = average_pitch;
            Some(calibration.clone())
        } else {
            None
        }
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
        if ack.error_code() == 7 {
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

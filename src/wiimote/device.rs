use std::ffi::CString;

use crate::prelude::*;
use crate::wiimote::simple_io;

use super::calibration::normalize;

#[derive(Debug, Clone, Copy)]
pub enum WiimoteDeviceType {
    /// Old Wii remote with potentially an external motion plus
    Wiimote = 0,
    /// Wii remote plus with integrated motion plus
    WiimotePlus = 1,
}

#[repr(C)]
pub struct NativeWiimote {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<core::marker::PhantomPinned>,
}

extern "C" {
    fn wiimote_read(wiimote: *mut NativeWiimote, buffer: *mut u8, buffer_size: usize) -> i32;
    fn wiimote_read_timeout(
        wiimote: *mut NativeWiimote,
        buffer: *mut u8,
        buffer_size: usize,
        timeout_millis: usize,
    ) -> i32;
    fn wiimote_write(wiimote: *mut NativeWiimote, buffer: *const u8, data_size: usize) -> i32;
    fn wiimote_get_identifier_length(wiimote: *mut NativeWiimote) -> usize;
    fn wiimote_get_identifier(
        wiimote: *mut NativeWiimote,
        identifier: *mut u8,
        identifier_buffer_length: usize,
    ) -> bool;
    fn wiimote_cleanup(wiimote: *mut NativeWiimote);
}

#[derive(Debug, Default, Clone)]
pub struct AccelerometerCalibration {
    x_zero_offset: u16,
    y_zero_offset: u16,
    z_zero_offset: u16,
    x_gravity: u16,
    y_gravity: u16,
    z_gravity: u16,
}

impl AccelerometerCalibration {
    #[must_use]
    pub fn get_acceleration(&self, data: &AccelerometerData) -> (f64, f64, f64) {
        let x = normalize(data.x, 10, self.x_zero_offset, self.x_gravity, 10);
        let y = normalize(data.y, 10, self.y_zero_offset, self.y_gravity, 10);
        let z = normalize(data.z, 10, self.z_zero_offset, self.z_gravity, 10);
        (x, y, z)
    }
}

pub struct AccelerometerData {
    x: u16,
    y: u16,
    z: u16,
}

impl AccelerometerData {
    /// The first two bytes are button data, the next three bytes are acceleration data.
    #[must_use]
    pub const fn from_normal_reporting(data: &[u8]) -> Self {
        Self {
            x: ((data[2] as u16) << 2) | (((data[0] as u16) >> 5) & 0b11),
            y: ((data[3] as u16) << 2) | (((data[1] as u16) >> 5) & 0b10),
            z: ((data[4] as u16) << 2) | (((data[1] as u16) >> 6) & 0b10),
        }
    }

    /// The first two bytes are button data, the next byte is acceleration data.
    #[must_use]
    #[allow(clippy::similar_names)]
    pub const fn from_interleaved_reporting(data_3e: &[u8], data_3f: &[u8]) -> Self {
        Self {
            x: (data_3e[2] as u16) << 2,
            y: (data_3f[2] as u16) << 2,
            z: (((data_3e[1] as u16) << 1) & 0b1100_0000)
                | (((data_3e[0] as u16) >> 1) & 0b0011_0000)
                | (((data_3f[1] as u16) >> 3) & 0b0000_1100)
                | (((data_3f[0] as u16) >> 5) & 0b0000_0011),
        }
    }
}

pub struct WiimoteDevice {
    device: *mut NativeWiimote,
    identifier: String,
    // device_type: WiimoteDeviceType,
    calibration_data: AccelerometerCalibration,
    motion_plus: Option<MotionPlus>,
    extension: Option<WiimoteExtension>,
}

unsafe impl Sync for WiimoteDevice {}
unsafe impl Send for WiimoteDevice {}

impl WiimoteDevice {
    /// Wraps the `DeviceInfo` as a `WiimoteDevice`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the device is not a recognized Wii remote or initialization failed.
    pub(crate) unsafe fn new(device: *mut NativeWiimote) -> WiimoteResult<Self> {
        if device.is_null() {
            return Err(WiimoteError::Disconnected);
        }

        let identifier = Self::get_identifier(device);
        println!("wiimote {identifier}");

        let mut wiimote = Self {
            device,
            identifier,
            // device_type,
            calibration_data: AccelerometerCalibration::default(),
            motion_plus: None,
            extension: None,
        };

        wiimote.initialize()?;
        Ok(wiimote)
    }

    pub(crate) fn get_identifier(device: *mut NativeWiimote) -> String {
        unsafe {
            let length = wiimote_get_identifier_length(device);
            let mut identifier = vec![0u8; length];
            if wiimote_get_identifier(device, identifier.as_mut_ptr(), identifier.len()) {
                CString::from_vec_with_nul(identifier)
                    .map_or(String::default(), |s| s.to_string_lossy().to_string())
            } else {
                String::default()
            }
        }
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    // #[must_use]
    // pub const fn device_type(&self) -> WiimoteDeviceType {
    //     self.device_type
    // }

    #[must_use]
    pub const fn accelerometer_calibration(&self) -> &AccelerometerCalibration {
        &self.calibration_data
    }

    #[must_use]
    pub const fn motion_plus(&self) -> Option<&MotionPlus> {
        self.motion_plus.as_ref()
    }

    #[must_use]
    pub const fn extension(&self) -> Option<&WiimoteExtension> {
        self.extension.as_ref()
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        !self.device.is_null()
    }

    pub fn disconnected(&mut self) {
        self.device = std::ptr::null_mut();
    }

    /// Reconnects the Wii remote from a `NativeWiimote`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the device is not a recognized Wii remote or the Wii remote failed to initialize.
    pub fn reconnect(&mut self, device: *mut NativeWiimote) -> WiimoteResult<()> {
        if !self.device.is_null() {
            unsafe {
                wiimote_cleanup(self.device);
            }
        }
        self.device = device;
        self.initialize()
    }

    /// Writes the data to the connected Wii remote.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or write failed.
    pub fn write(&self, data: &[u8]) -> WiimoteResult<usize> {
        if self.device.is_null() {
            return Err(WiimoteError::Disconnected);
        }

        let written = unsafe { wiimote_write(self.device, data.as_ptr(), data.len()) };
        if written >= 0 {
            #[allow(clippy::cast_sign_loss)]
            Ok(written as usize)
        } else {
            Err(WiimoteError::Disconnected)
        }
    }

    /// Reads data from the connected Wii remote.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or read failed.
    pub fn read(&self, buffer: &mut [u8]) -> WiimoteResult<usize> {
        if self.device.is_null() {
            return Err(WiimoteError::Disconnected);
        }

        let read = unsafe { wiimote_read(self.device, buffer.as_mut_ptr(), buffer.len()) };
        if read >= 0 {
            #[allow(clippy::cast_sign_loss)]
            Ok(read as usize)
        } else {
            Err(WiimoteError::Disconnected)
        }
    }

    /// Reads data from the connected Wii remote waiting for a maximum of `timeout_millis`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or read failed.
    pub fn read_timeout(&self, buffer: &mut [u8], timeout_millis: usize) -> WiimoteResult<usize> {
        if self.device.is_null() {
            return Err(WiimoteError::Disconnected);
        }

        let read = unsafe {
            wiimote_read_timeout(
                self.device,
                buffer.as_mut_ptr(),
                buffer.len(),
                timeout_millis,
            )
        };
        if read >= 0 {
            #[allow(clippy::cast_sign_loss)]
            Ok(read as usize)
        } else {
            Err(WiimoteError::Disconnected)
        }
    }

    fn initialize(&mut self) -> WiimoteResult<()> {
        self.motion_plus = None;
        self.extension = None;

        self.calibration_data = self.read_calibration_data()?;
        self.motion_plus = MotionPlus::detect(self)?;
        self.extension = WiimoteExtension::detect(self)?;
        Ok(())
    }

    fn read_calibration_data(&self) -> WiimoteResult<AccelerometerCalibration> {
        // https://www.wiibrew.org/wiki/Wiimote#EEPROM_Memory
        // The four bytes starting at 0x0016 and 0x0020 store the calibrated zero offsets for the accelerometer
        // (high 8 bits of X,Y,Z in the first three bytes, low 2 bits packed in the fourth byte as --XXYYZZ).
        // The four bytes at 0x001A and 0x24 store the force of gravity on those axes.
        let data = simple_io::read_16_bytes_sync_checked(self, Addressing::eeprom(0x0016, 10))?;

        let mut checksum = 0x55u8;
        for byte in &data[..9] {
            checksum = checksum.wrapping_add(*byte);
        }
        if checksum != data[9] {
            return Err(WiimoteDeviceError::InvalidChecksum.into());
        }

        Ok(AccelerometerCalibration {
            x_zero_offset: ((data[0] as u16) << 2) | ((data[3] as u16) >> 4 & 0b11),
            y_zero_offset: ((data[1] as u16) << 2) | ((data[3] as u16) >> 2 & 0b11),
            z_zero_offset: ((data[2] as u16) << 2) | ((data[3] as u16) & 0b11),
            x_gravity: ((data[4] as u16) << 2) | ((data[7] as u16) >> 4 & 0b11),
            y_gravity: ((data[5] as u16) << 2) | ((data[7] as u16) >> 2 & 0b11),
            z_gravity: ((data[6] as u16) << 2) | ((data[7] as u16) & 0b11),
        })
    }
}

impl Drop for WiimoteDevice {
    fn drop(&mut self) {
        if !self.device.is_null() {
            unsafe {
                wiimote_cleanup(self.device);
            }
        }

        self.device = std::ptr::null_mut();
    }
}

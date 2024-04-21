use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::calibration::normalize;
use crate::extensions::{MotionPlus, WiimoteExtension};
use crate::input::InputReport;
use crate::native::{NativeWiimote, NativeWiimoteDevice};
use crate::output::{Addressing, OutputReport};
use crate::prelude::*;
use crate::simple_io;

/// The calibration data for the accelerometer of the Wii remote.
/// Can be used to convert raw accelerometer data to acceleration values.
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
    /// Returns the acceleration values from the raw data using the current calibration.
    #[must_use]
    pub fn get_acceleration(&self, data: &AccelerometerData) -> (f64, f64, f64) {
        let x = normalize(data.x, 10, self.x_zero_offset, self.x_gravity, 10);
        let y = normalize(data.y, 10, self.y_zero_offset, self.y_gravity, 10);
        let z = normalize(data.z, 10, self.z_zero_offset, self.z_gravity, 10);
        (x, y, z)
    }
}

/// The raw accelerometer data from the Wii remote.
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

/// A `WiimoteDevice` can be used to communicate with a Wii remote.
pub struct WiimoteDevice {
    device: Mutex<Option<NativeWiimoteDevice>>,
    identifier: String,
    calibration_data: AccelerometerCalibration,
    motion_plus: Option<MotionPlus>,
    extension: Option<WiimoteExtension>,
    rumble_enabled: AtomicBool,
}

unsafe impl Sync for WiimoteDevice {}
unsafe impl Send for WiimoteDevice {}

impl WiimoteDevice {
    /// Wraps the `NativeWiimoteDevice` as a `WiimoteDevice`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the device is not a recognized Wii remote or initialization failed.
    pub(crate) fn new(device: NativeWiimoteDevice) -> WiimoteResult<Self> {
        let identifier = device.identifier();
        let mut wiimote = Self {
            device: Mutex::new(Some(device)),
            identifier,
            calibration_data: AccelerometerCalibration::default(),
            motion_plus: None,
            extension: None,
            rumble_enabled: AtomicBool::new(false),
        };

        wiimote.initialize()?;
        Ok(wiimote)
    }

    /// Returns the unique identifier of the Wii remote.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Returns the accelerometer calibration data of the Wii remote.
    /// This data is used to convert raw accelerometer data to acceleration values.
    #[must_use]
    pub const fn accelerometer_calibration(&self) -> &AccelerometerCalibration {
        &self.calibration_data
    }

    /// Returns the `MotionPlus` extension of the Wii remote if connected.
    #[must_use]
    pub const fn motion_plus(&self) -> Option<&MotionPlus> {
        self.motion_plus.as_ref()
    }

    /// Returns data about the Wii remote extension if connected.
    #[must_use]
    pub const fn extension(&self) -> Option<&WiimoteExtension> {
        self.extension.as_ref()
    }

    /// Returns whether the Wii remote is currently connected.
    /// The Wii remote is automatically re-assigned to this object when reconnected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.device
            .lock()
            .map(|device| device.is_some())
            .unwrap_or(false)
    }

    /// Reconnects the Wii remote from a `NativeWiimoteDevice`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the device is not a recognized Wii remote or the Wii remote failed to initialize.
    pub fn reconnect(&mut self, device: NativeWiimoteDevice) -> WiimoteResult<()> {
        self.disconnected();
        _ = self.device.lock().map(|mut d| d.replace(device));
        self.initialize()
    }

    /// Writes the data to the connected Wii remote.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or write failed.
    pub fn write(&self, output_report: &OutputReport) -> WiimoteResult<()> {
        let mut device = match self.device.lock() {
            Ok(device) => device,
            Err(err) => err.into_inner(),
        };
        if let Some(device) = device.as_mut() {
            let rumble = if let OutputReport::Rumble(new_rumble) = output_report {
                // Rumble is sent in every output report, so the new value needs to be stored.
                self.rumble_enabled.store(*new_rumble, Ordering::Relaxed);
                *new_rumble
            } else {
                self.rumble_enabled.load(Ordering::Relaxed)
            };
            let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];
            let size = output_report.fill_buffer(rumble, &mut buffer);
            if device.write(&buffer[..size]).is_some() {
                return Ok(());
            }
        }
        _ = device.take();
        Err(WiimoteError::Disconnected)
    }

    /// Reads data from the connected Wii remote.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or read failed.
    pub fn read(&self) -> WiimoteResult<InputReport> {
        let mut device = match self.device.lock() {
            Ok(device) => device,
            Err(err) => err.into_inner(),
        };
        if let Some(device) = device.as_mut() {
            let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];
            if let Some(bytes_read) = device.read(&mut buffer) {
                return InputReport::try_from(&buffer[..bytes_read]);
            }
        }
        _ = device.take();
        Err(WiimoteError::Disconnected)
    }

    /// Reads data from the connected Wii remote waiting for a maximum of `timeout_millis`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or read failed.
    pub fn read_timeout(&self, timeout_millis: usize) -> WiimoteResult<InputReport> {
        let mut device = match self.device.lock() {
            Ok(device) => device,
            Err(err) => err.into_inner(),
        };
        if let Some(device) = device.as_mut() {
            let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];
            if let Some(bytes_read) = device.read_timeout(&mut buffer, timeout_millis) {
                return InputReport::try_from(&buffer[..bytes_read]);
            }
        }
        _ = device.take();
        Err(WiimoteError::Disconnected)
    }

    fn initialize(&mut self) -> WiimoteResult<()> {
        self.motion_plus = None;
        self.extension = None;

        self.calibration_data = self.read_calibration_data()?;
        self.motion_plus = MotionPlus::detect(self)?;
        self.extension = WiimoteExtension::detect(self)?;
        Ok(())
    }

    fn read_calibration_data(&mut self) -> WiimoteResult<AccelerometerCalibration> {
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

    fn disconnected(&self) {
        _ = self.device.lock().map(|mut device| device.take());
    }
}

impl Drop for WiimoteDevice {
    fn drop(&mut self) {
        self.disconnected();
    }
}

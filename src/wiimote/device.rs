use crate::prelude::*;
use crate::wiimote::simple_io;

#[derive(Debug, Clone, Copy)]
pub enum WiimoteDeviceType {
    /// Old Wii remote with potentially an external motion plus
    Wiimote = 0,
    /// Wii remote plus with integrated motion plus
    WiimotePlus = 1,
}

#[derive(Debug, Default)]
struct AccelererometerCalibrationData {
    x_zero_offset: u16,
    y_zero_offset: u16,
    z_zero_offset: u16,
    x_gravity: u16,
    y_gravity: u16,
    z_gravity: u16,
}

pub struct WiimoteDevice {
    hid_device: Option<HidDevice>,
    serial_number: String,
    device_type: WiimoteDeviceType,
    calibration_data: AccelererometerCalibrationData,
}

impl WiimoteDevice {
    pub const VENDOR_ID: u16 = 0x057E;
    pub const PRODUCT_ID_WIIMOTE: u16 = 0x0306; // RVL-003
    pub const PRODUCT_ID_WIIMOTE_PLUS: u16 = 0x0330; // RVL-036

    /// Wraps the `DeviceInfo` as a `WiimoteDevice`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the device is not a recognized Wii remote.
    pub fn new(device_info: &DeviceInfo, hid_api: &HidApi) -> WiimoteResult<Self> {
        let device_type = Self::get_wiimote_device_type(device_info)?;

        let serial = device_info.serial_number().unwrap_or("");
        let hid_device = device_info.open_device(hid_api)?;

        let mut wiimote = Self {
            hid_device: Some(hid_device),
            serial_number: serial.to_string(),
            device_type,
            calibration_data: AccelererometerCalibrationData::default(),
        };

        wiimote.initialize()?;
        Ok(wiimote)
    }

    /// Checks that the device is a Wii remote.
    ///
    /// # Errors
    ///
    /// This function will return an error if the device is not a recognized Wii remote.
    pub fn get_wiimote_device_type(device_info: &DeviceInfo) -> WiimoteResult<WiimoteDeviceType> {
        if device_info.vendor_id() != Self::VENDOR_ID {
            return Err(WiimoteDeviceError::InvalidVendorID(device_info.vendor_id()).into());
        }

        match device_info.product_id() {
            Self::PRODUCT_ID_WIIMOTE => Ok(WiimoteDeviceType::Wiimote),
            Self::PRODUCT_ID_WIIMOTE_PLUS => Ok(WiimoteDeviceType::WiimotePlus),
            product_id => Err(WiimoteDeviceError::InvalidProductID(product_id).into()),
        }
    }

    #[must_use]
    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    #[must_use]
    pub const fn device_type(&self) -> WiimoteDeviceType {
        self.device_type
    }

    #[must_use]
    pub const fn is_connected(&self) -> bool {
        self.hid_device.is_some()
    }

    pub(crate) fn disconnected(&mut self) {
        self.hid_device = None;
    }

    pub(crate) fn reconnect(
        &mut self,
        device_info: &DeviceInfo,
        hid_api: &HidApi,
    ) -> WiimoteResult<()> {
        let device_type = Self::get_wiimote_device_type(device_info)?;
        let hid_device = device_info.open_device(hid_api)?;
        self.device_type = device_type;
        self.hid_device = Some(hid_device);
        self.initialize()?;
        Ok(())
    }

    /// Writes the data to the connected Wii remote.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or write failed.
    pub fn write(&self, data: &[u8]) -> WiimoteResult<usize> {
        if let Some(hid_device) = &self.hid_device {
            Ok(hid_device.write(data)?)
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
        if let Some(hid_device) = &self.hid_device {
            Ok(hid_device.read(buffer)?)
        } else {
            Err(WiimoteError::Disconnected)
        }
    }

    /// Reads data from the connected Wii remote waiting for a maximum of `timeout_millis`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the Wii remote is disconnected or read failed.
    pub fn read_timeout(&self, buf: &mut [u8], timeout_millis: i32) -> WiimoteResult<usize> {
        if let Some(hid_device) = &self.hid_device {
            Ok(hid_device.read_timeout(buf, timeout_millis)?)
        } else {
            Err(WiimoteError::Disconnected)
        }
    }

    fn initialize(&mut self) -> WiimoteResult<()> {
        self.calibration_data = self.read_calibration_data()?;
        Ok(())
    }

    fn read_calibration_data(&self) -> WiimoteResult<AccelererometerCalibrationData> {
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

        Ok(AccelererometerCalibrationData {
            x_zero_offset: ((data[0] as u16) << 2) | ((data[3] as u16) >> 4 & 0b11),
            y_zero_offset: ((data[1] as u16) << 2) | ((data[3] as u16) >> 2 & 0b11),
            z_zero_offset: ((data[2] as u16) << 2) | ((data[3] as u16) & 0b11),
            x_gravity: ((data[4] as u16) << 2) | ((data[7] as u16) >> 4 & 0b11),
            y_gravity: ((data[5] as u16) << 2) | ((data[7] as u16) >> 2 & 0b11),
            z_gravity: ((data[6] as u16) << 2) | ((data[7] as u16) & 0b11),
        })
    }
}

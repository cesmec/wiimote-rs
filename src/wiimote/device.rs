use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum WiimoteDeviceType {
    /// Old Wii remote with potentially an external motion plus
    Wiimote = 0,
    /// Wii remote plus with integrated motion plus
    WiimotePlus = 1,
}

pub struct WiimoteDevice {
    hid_device: Option<HidDevice>,
    serial_number: String,
    device_type: WiimoteDeviceType,
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

        Ok(Self {
            hid_device: Some(hid_device),
            serial_number: serial.to_string(),
            device_type,
        })
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
}

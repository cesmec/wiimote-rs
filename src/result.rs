use hidapi::HidError;

#[derive(Debug)]
pub enum WiimoteError {
    HidApiError(HidError),
    WiimoteDeviceError(WiimoteDeviceError),
    Disconnected,
}

impl From<HidError> for WiimoteError {
    fn from(e: HidError) -> Self {
        Self::HidApiError(e)
    }
}

#[derive(Debug)]
pub enum WiimoteDeviceError {
    InvalidVendorID(u16),
    InvalidProductID(u16),
    InvalidData,
}

impl From<WiimoteDeviceError> for WiimoteError {
    fn from(e: WiimoteDeviceError) -> Self {
        Self::WiimoteDeviceError(e)
    }
}

pub type WiimoteResult<T> = Result<T, WiimoteError>;

#[derive(Debug)]
pub enum WiimoteError {
    WiimoteDeviceError(WiimoteDeviceError),
    Disconnected,
}

#[derive(Debug)]
pub enum WiimoteDeviceError {
    InvalidVendorID(u16),
    InvalidProductID(u16),
    MissingData,
    InvalidChecksum,
    InvalidData,
}

impl From<WiimoteDeviceError> for WiimoteError {
    fn from(e: WiimoteDeviceError) -> Self {
        Self::WiimoteDeviceError(e)
    }
}

pub type WiimoteResult<T> = Result<T, WiimoteError>;

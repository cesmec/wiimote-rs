mod calibration;
mod device;
pub mod extensions;
pub mod input;
mod manager;
mod output;
mod simple_io;

pub use device::{AccelerometerCalibration, AccelerometerData, WiimoteDevice, WiimoteDeviceType};
pub use extensions::*;
pub use input::InputReport;
pub use manager::WiimoteManager;
pub use output::*;

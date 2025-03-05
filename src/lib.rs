#![allow(clippy::module_name_repetitions)]

mod calibration;
mod device;
pub mod extensions;
pub mod input;
mod manager;
mod native;
pub mod output;
mod result;
pub mod simple_io;

pub const WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE: usize = 32;

pub mod prelude {
    pub use crate::device::{AccelerometerCalibration, AccelerometerData, WiimoteDevice};
    pub use crate::extensions::motion_plus::*;
    pub use crate::manager::WiimoteManager;
    pub use crate::result::*;
    pub use crate::WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE;
}

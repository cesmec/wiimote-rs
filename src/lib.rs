#![allow(clippy::module_name_repetitions)]

mod result;
mod wiimote;

pub const WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE: usize = 32;

pub mod prelude {
    pub use crate::result::*;
    pub use crate::wiimote::*;
    pub use crate::WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE;
}

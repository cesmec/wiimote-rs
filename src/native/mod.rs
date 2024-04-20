mod common;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{wiimotes_scan, wiimotes_scan_cleanup, LinuxNativeWiimote as NativeWiimoteDevice};

#[cfg(target_os = "windows")]
pub use windows::{
    wiimotes_scan, wiimotes_scan_cleanup, WindowsNativeWiimote as NativeWiimoteDevice,
};

pub trait NativeWiimote {
    fn read(&mut self, buffer: &mut [u8]) -> Option<usize>;
    fn read_timeout(&mut self, buffer: &mut [u8], timeout_millis: usize) -> Option<usize>;
    fn write(&mut self, buffer: &[u8]) -> Option<usize>;
    fn identifier(&self) -> String;
}

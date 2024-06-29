mod bluetooth;
mod hid;

use std::collections::HashSet;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use windows::Win32::Devices::HumanInterfaceDevice::HIDP_CAPS;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE, WAIT_FAILED,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Globalization::{WideCharToMultiByte, CP_UTF8};
use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject, INFINITE};
use windows::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};

use self::bluetooth::{disconnect_wiimotes, forget_wiimote, register_wiimotes_as_hid_devices};
use self::hid::{enumerate_wiimote_hid_devices, open_wiimote_device};

use super::NativeWiimote;

static mut WIIMOTES_HANDLED: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

unsafe fn from_wstring(wstr: &[u16]) -> String {
    if wstr.is_empty() {
        return String::new();
    }
    let length = wstr.iter().position(|&c| c == 0).unwrap_or(wstr.len());
    let result_size = WideCharToMultiByte(CP_UTF8, 0, &wstr[..length], None, None, None);
    if result_size <= 0 {
        return String::new();
    }

    #[allow(clippy::cast_sign_loss)]
    let mut result = vec![0u8; result_size as usize];
    WideCharToMultiByte(CP_UTF8, 0, wstr, Some(&mut result), None, None);
    String::from_utf8_unchecked(result)
}

pub fn wiimotes_scan(wiimotes: &mut Vec<WindowsNativeWiimote>) {
    unsafe {
        _ = register_wiimotes_as_hid_devices();

        _ = enumerate_wiimote_hid_devices(|device_info, device_path| {
            let mut wiimotes_handled = match WIIMOTES_HANDLED.lock() {
                Ok(wiimotes_handled) => wiimotes_handled,
                Err(wiimotes_handled) => wiimotes_handled.into_inner(),
            };

            if !wiimotes_handled.contains(device_info.serial_number()) {
                open_wiimote_device(device_path, (GENERIC_READ | GENERIC_WRITE).0).map_or_else(
                    |_| {
                        eprintln!("Failed to connect to wiimote");
                    },
                    |wiimote_handle| {
                        let serial_number = device_info.serial_number();
                        wiimotes_handled.insert(serial_number.to_string());
                        wiimotes.push(WindowsNativeWiimote::new(
                            wiimote_handle,
                            serial_number.to_string(),
                            device_info.capabilities(),
                        ));
                    },
                );
            }
        });
    }
}

pub fn wiimotes_scan_cleanup() {
    unsafe {
        disconnect_wiimotes();
    }
}

pub struct WindowsNativeWiimote {
    handle: HANDLE,
    identifier: String,
    read_pending: bool,
    write_pending: bool,
    overlapped_read: OVERLAPPED,
    overlapped_write: OVERLAPPED,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl WindowsNativeWiimote {
    fn new(handle: HANDLE, identifier: String, capabilities: &HIDP_CAPS) -> Self {
        let read_buffer_size = capabilities.InputReportByteLength as usize;
        let write_buffer_size = capabilities.OutputReportByteLength as usize;
        let mut wiimote = Self {
            handle,
            identifier,
            read_pending: false,
            write_pending: false,
            overlapped_read: OVERLAPPED::default(),
            overlapped_write: OVERLAPPED::default(),
            read_buffer: vec![0; read_buffer_size],
            write_buffer: vec![0; write_buffer_size],
        };
        wiimote.overlapped_read.hEvent = unsafe { CreateEventW(None, true, false, None).unwrap() };
        wiimote.overlapped_write.hEvent = unsafe { CreateEventW(None, true, false, None).unwrap() };
        wiimote
    }

    unsafe fn read_timeout_impl(
        &mut self,
        buffer: &mut [u8],
        timeout_millis: Option<usize>,
    ) -> Option<usize> {
        let buffer_size = usize::min(buffer.len(), self.read_buffer.len());
        let mut did_read = false;
        if !self.read_pending {
            _ = ResetEvent(self.overlapped_read.hEvent);
            self.read_buffer.fill(0);
            did_read = ReadFile(
                self.handle,
                Some(&mut self.read_buffer),
                None,
                Some(&mut self.overlapped_read),
            )
            .is_ok();
            if !did_read && GetLastError() != ERROR_IO_PENDING {
                return None;
            }

            self.read_pending = true;
        }

        if !did_read && timeout_millis.is_some() {
            let wait_result =
                WaitForSingleObject(self.overlapped_read.hEvent, timeout_millis.unwrap() as u32);
            if wait_result == WAIT_TIMEOUT {
                return Some(0);
            }
            if wait_result != WAIT_OBJECT_0 {
                // Wait failed
                return None;
            }
        }

        let mut bytes_read = 0;
        let result =
            GetOverlappedResult(self.handle, &self.overlapped_read, &mut bytes_read, true).is_ok();
        self.read_pending = false;
        if result {
            let bytes_to_copy = usize::min(bytes_read as usize, buffer_size);
            buffer[..bytes_to_copy].copy_from_slice(&self.read_buffer[..bytes_to_copy]);
            Some(bytes_to_copy)
        } else {
            None
        }
    }

    unsafe fn write_impl(&mut self, buffer: &[u8]) -> Option<usize> {
        if self.write_pending {
            WaitForSingleObject(self.overlapped_write.hEvent, INFINITE);
        }
        self.write_pending = true;

        let data_size = usize::min(buffer.len(), self.write_buffer.len());
        self.write_buffer[..data_size].copy_from_slice(&buffer[..data_size]);
        self.write_buffer[data_size..].fill(0);

        if WriteFile(
            self.handle,
            Some(&self.write_buffer),
            None,
            Some(&mut self.overlapped_write),
        )
        .is_err()
        {
            if GetLastError() != ERROR_IO_PENDING {
                return None;
            }

            let wait_result = WaitForSingleObject(self.overlapped_write.hEvent, INFINITE);
            if wait_result != WAIT_OBJECT_0 {
                self.write_pending = false;
                if wait_result == WAIT_FAILED {
                    println!("error: {}", GetLastError().0);
                }
                return None;
            }
        }

        self.write_pending = false;
        let mut bytes_written = 0;
        if GetOverlappedResult(
            self.handle,
            &self.overlapped_write,
            &mut bytes_written,
            true,
        )
        .is_err()
        {
            None
        } else {
            Some(bytes_written as usize)
        }
    }
}

impl NativeWiimote for WindowsNativeWiimote {
    fn read(&mut self, buffer: &mut [u8]) -> Option<usize> {
        unsafe { self.read_timeout_impl(buffer, None) }
    }

    fn read_timeout(&mut self, buffer: &mut [u8], timeout_millis: usize) -> Option<usize> {
        unsafe { self.read_timeout_impl(buffer, Some(timeout_millis)) }
    }

    fn write(&mut self, buffer: &[u8]) -> Option<usize> {
        unsafe { self.write_impl(buffer) }
    }

    fn identifier(&self) -> String {
        self.identifier.clone()
    }
}

impl Drop for WindowsNativeWiimote {
    fn drop(&mut self) {
        unsafe {
            _ = CloseHandle(self.overlapped_read.hEvent);
            _ = CloseHandle(self.overlapped_write.hEvent);
            _ = CloseHandle(self.handle);

            forget_wiimote(&self.identifier);
            let mut wiimotes_handled = match WIIMOTES_HANDLED.lock() {
                Ok(wiimotes_handled) => wiimotes_handled,
                Err(wiimotes_handled) => wiimotes_handled.into_inner(),
            };
            wiimotes_handled.remove(&self.identifier);
        }
    }
}

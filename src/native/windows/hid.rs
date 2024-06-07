use std::collections::HashSet;
use std::ffi::c_void;
use std::{iter, mem};

use once_cell::sync::Lazy;
use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_Interface_ListW, CM_Get_Device_Interface_List_SizeW,
    CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CR_SUCCESS,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidD_GetAttributes, HidD_GetHidGuid, HidD_GetPreparsedData, HidD_GetSerialNumberString,
    HidP_GetCaps, HIDD_ATTRIBUTES, HIDP_CAPS, HIDP_STATUS_SUCCESS, PHIDP_PREPARSED_DATA,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

use crate::native::common::is_wiimote;

use super::from_wstring;

pub(super) struct DeviceInfo {
    vendor_id: u16,
    product_id: u16,
    serial_number: String,
    capabilities: HIDP_CAPS,
}

impl DeviceInfo {
    pub(super) const fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    pub(super) const fn product_id(&self) -> u16 {
        self.product_id
    }

    pub(super) fn serial_number(&self) -> &str {
        &self.serial_number
    }

    pub(super) const fn capabilities(&self) -> &HIDP_CAPS {
        &self.capabilities
    }

    unsafe fn from_device_path(device_path: &str) -> Option<Self> {
        let device_handle = open_wiimote_device(device_path, 0).ok()?;
        let mut attributes = HIDD_ATTRIBUTES {
            Size: mem::size_of::<HIDD_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        let mut name_buffer = [0u16; 64];
        let mut preparsed_data: PHIDP_PREPARSED_DATA = PHIDP_PREPARSED_DATA::default();
        let mut capabilities = HIDP_CAPS::default();
        let device_info = if HidD_GetAttributes(device_handle, &mut attributes).as_bool()
            && HidD_GetSerialNumberString(
                device_handle,
                name_buffer.as_mut_ptr().cast::<c_void>(),
                mem::size_of_val(&name_buffer) as u32,
            )
            .as_bool()
            && HidD_GetPreparsedData(device_handle, &mut preparsed_data).as_bool()
            && HidP_GetCaps(preparsed_data, &mut capabilities) == HIDP_STATUS_SUCCESS
        {
            Some(Self {
                vendor_id: attributes.VendorID,
                product_id: attributes.ProductID,
                serial_number: from_wstring(&name_buffer),
                capabilities,
            })
        } else {
            None
        };
        _ = CloseHandle(device_handle);
        device_info
    }
}

pub(super) unsafe fn open_wiimote_device(
    device_path: &str,
    access: u32,
) -> Result<HANDLE, windows::core::Error> {
    let share_read_write = FILE_SHARE_READ | FILE_SHARE_WRITE;
    let device_path_utf16: Vec<u16> = device_path.encode_utf16().chain(iter::once(0)).collect();
    CreateFileW(
        PCWSTR(device_path_utf16.as_ptr()),
        access,
        share_read_write,
        None,
        OPEN_EXISTING,
        FILE_FLAG_OVERLAPPED,
        None,
    )
}

pub(super) unsafe fn enumerate_wiimote_hid_devices<F>(mut callback: F) -> Result<(), String>
where
    F: FnMut(&DeviceInfo, &str),
{
    static mut UNRELATED_DEVICES: Lazy<HashSet<String>> = Lazy::new(HashSet::new);

    let hid_id = HidD_GetHidGuid();

    let mut length = 0;
    let config_ret = CM_Get_Device_Interface_List_SizeW(
        &mut length,
        &hid_id,
        PCWSTR(std::ptr::null()),
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
    );
    if config_ret != CR_SUCCESS {
        return Err(String::from("Failed to get HID device list size"));
    }

    let mut device_list = vec![0u16; length as usize];
    let config_ret = CM_Get_Device_Interface_ListW(
        &hid_id,
        PCWSTR(std::ptr::null()),
        &mut device_list,
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
    );
    if config_ret != CR_SUCCESS {
        return Err(String::from("Failed to get HID device list"));
    }

    let mut start_index = 0;
    while let Some(device_path_length) = device_list[start_index..].iter().position(|&c| c == 0) {
        if device_list[start_index] == 0 {
            break;
        }
        let end_index = start_index + device_path_length + 1;

        let device_path = &device_list[start_index..end_index];
        let device_path_string = from_wstring(device_path);
        start_index = end_index;
        if UNRELATED_DEVICES.contains(&device_path_string) {
            continue;
        }

        if let Some(device_info) = DeviceInfo::from_device_path(&device_path_string) {
            if is_wiimote(device_info.vendor_id(), device_info.product_id()) {
                callback(&device_info, &device_path_string);
            } else {
                UNRELATED_DEVICES.insert(device_path_string);
            }
        }
    }
    Ok(())
}

use std::collections::HashMap;
use std::mem;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use windows::Win32::Devices::Bluetooth::{
    BluetoothFindDeviceClose, BluetoothFindFirstDevice, BluetoothFindFirstRadio,
    BluetoothFindNextDevice, BluetoothFindNextRadio, BluetoothFindRadioClose,
    BluetoothGetRadioInfo, BluetoothRemoveDevice, BluetoothSetServiceState, BLUETOOTH_DEVICE_INFO,
    BLUETOOTH_DEVICE_SEARCH_PARAMS, BLUETOOTH_FIND_RADIO_PARAMS, BLUETOOTH_RADIO_INFO,
    BLUETOOTH_SERVICE_DISABLE, BLUETOOTH_SERVICE_ENABLE,
};
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, TRUE};

use crate::native::common::is_wiimote_device_name;

use super::from_wstring;

const HUMAN_INTERFACE_DEVICE_SERVICE_CLASS_ID: u128 = 0x1124_0000_1000_8000_0080_5F9B_34FB;

static mut CONNECTED_WIIMOTES: Lazy<Mutex<HashMap<String, BLUETOOTH_DEVICE_INFO>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

unsafe fn enumerate_bluetooth_radios<F>(mut callback: F) -> Result<(), String>
where
    F: FnMut(HANDLE, &BLUETOOTH_RADIO_INFO),
{
    let mut radio_param = BLUETOOTH_FIND_RADIO_PARAMS::default();
    radio_param.dwSize = mem::size_of_val(&radio_param) as u32;

    let mut radio = HANDLE::default();
    let radio_find = BluetoothFindFirstRadio(&radio_param, &mut radio)
        .map_err(|_| String::from("No bluetooth adapter found"))?;

    loop {
        let mut radio_info = BLUETOOTH_RADIO_INFO::default();
        radio_info.dwSize = mem::size_of_val(&radio_info) as u32;

        if BluetoothGetRadioInfo(radio, &mut radio_info) == ERROR_SUCCESS.0 {
            callback(radio, &radio_info);
        }
        _ = CloseHandle(radio);
        if BluetoothFindNextRadio(radio_find, &mut radio).is_err() {
            break;
        }
    }

    _ = BluetoothFindRadioClose(radio_find);
    Ok(())
}

unsafe fn enumerate_bluetooth_devices<F>(
    search: &mut BLUETOOTH_DEVICE_SEARCH_PARAMS,
    callback: F,
) -> Result<(), String>
where
    F: Fn(HANDLE, &BLUETOOTH_RADIO_INFO, &BLUETOOTH_DEVICE_INFO),
{
    enumerate_bluetooth_radios(|radio, radio_info| {
        search.hRadio = radio;

        let mut device_info = BLUETOOTH_DEVICE_INFO::default();
        device_info.dwSize = mem::size_of_val(&device_info) as u32;

        if let Ok(device_find) = BluetoothFindFirstDevice(search, &mut device_info) {
            loop {
                callback(radio, radio_info, &device_info);
                if BluetoothFindNextDevice(device_find, &mut device_info).is_err() {
                    break;
                }
            }

            _ = BluetoothFindDeviceClose(device_find);
        }
    })
}

unsafe fn register_as_hid_device(
    radio: HANDLE,
    device_info: &BLUETOOTH_DEVICE_INFO,
) -> Result<(), String> {
    let device_id = format!("{:x}", device_info.Address.Anonymous.ullLong);
    let mut connected = match CONNECTED_WIIMOTES.lock() {
        Ok(connected) => connected,
        Err(connected) => connected.into_inner(),
    };
    if connected.contains_key(&device_id) {
        return Ok(());
    }

    if !device_info.fConnected.as_bool() && device_info.fRemembered.as_bool() {
        BluetoothRemoveDevice(&device_info.Address);
    }
    if device_info.fConnected.as_bool() || device_info.fRemembered.as_bool() {
        return Ok(());
    }

    let hid_serivce_class_guid = HUMAN_INTERFACE_DEVICE_SERVICE_CLASS_ID.into();

    let result = BluetoothSetServiceState(
        radio,
        device_info,
        &hid_serivce_class_guid,
        BLUETOOTH_SERVICE_ENABLE,
    );
    if result != ERROR_SUCCESS.0 {
        return Err(String::from(
            "Failed to register wiimote as interface device",
        ));
    }

    connected.insert(device_id, *device_info);
    Ok(())
}

pub(super) fn register_wiimotes_as_hid_devices() -> Result<(), String> {
    let mut search = BLUETOOTH_DEVICE_SEARCH_PARAMS::default();
    search.dwSize = mem::size_of_val(&search) as u32;
    search.fReturnAuthenticated = TRUE;
    search.fReturnRemembered = TRUE;
    search.fReturnUnknown = TRUE;
    search.fReturnConnected = TRUE;
    search.fIssueInquiry = TRUE;
    search.cTimeoutMultiplier = 2;

    unsafe {
        enumerate_bluetooth_devices(&mut search, |radio, _radio_info, device_info| {
            let name = from_wstring(&device_info.szName);
            if is_wiimote_device_name(&name) {
                if let Err(error) = register_as_hid_device(radio, device_info) {
                    eprintln!("Failed to register wiimote as interface device: {error}");
                }
            }
        })
    }
}

pub(super) fn disconnect_wiimote(identifier: &str) {
    unsafe {
        let mut connected_wiimotes = match CONNECTED_WIIMOTES.lock() {
            Ok(connected_wiimotes) => connected_wiimotes,
            Err(connected_wiimotes) => connected_wiimotes.into_inner(),
        };
        if let Some(connected_wiimote) = connected_wiimotes.remove(identifier) {
            _ = enumerate_bluetooth_radios(|radio, _radio_info| {
                let hid_guid = HUMAN_INTERFACE_DEVICE_SERVICE_CLASS_ID.into();
                BluetoothSetServiceState(
                    radio,
                    &connected_wiimote,
                    &hid_guid,
                    BLUETOOTH_SERVICE_DISABLE,
                );
            });
        }
    }
}

pub(super) unsafe fn disconnect_wiimotes() {
    _ = enumerate_bluetooth_radios(|radio, _radio_info| {
        let connected_wiimotes = match CONNECTED_WIIMOTES.lock() {
            Ok(connected_wiimotes) => connected_wiimotes,
            Err(connected_wiimotes) => connected_wiimotes.into_inner(),
        };
        let hid_guid = HUMAN_INTERFACE_DEVICE_SERVICE_CLASS_ID.into();
        for (_device_id, connected_wiimote) in connected_wiimotes.iter() {
            BluetoothSetServiceState(
                radio,
                connected_wiimote,
                &hid_guid,
                BLUETOOTH_SERVICE_DISABLE,
            );
        }
    });

    let mut connected_wiimotes = match CONNECTED_WIIMOTES.lock() {
        Ok(connected_wiimotes) => connected_wiimotes,
        Err(connected_wiimotes) => connected_wiimotes.into_inner(),
    };
    connected_wiimotes.clear();
}

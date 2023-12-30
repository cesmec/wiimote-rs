#include "hid.h"

#ifdef _WIN32

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <BluetoothAPIs.h>

#include <codecvt>
#include <iostream>
#include <locale>
#include <string>

std::wstring to_wstring(const std::string& str) {
    using convert_type = std::codecvt_utf8<wchar_t>;
    std::wstring_convert<convert_type, wchar_t> converter;

    return converter.from_bytes(str);
}

std::string from_wstring(const std::wstring& wstr) {
    using convert_type = std::codecvt_utf8<wchar_t>;
    std::wstring_convert<convert_type, wchar_t> converter;

    return converter.to_bytes(wstr);
}

bool is_wiimote_device_name(const std::string& name) {
    return name == "Nintendo RVL-CNT-01" || name == "Nintendo RVL-CNT-01-TR";
}

bool attach_wiimote(HANDLE radio, BLUETOOTH_DEVICE_INFO& device_info) {
    if (device_info.fConnected || device_info.fRemembered) {
        return false;
    }

    // Enable HID service
    DWORD result = BluetoothSetServiceState(
        radio,
        &device_info,
        &HumanInterfaceDeviceServiceClass_UUID,
        BLUETOOTH_SERVICE_ENABLE);

    if (FAILED(result)) {
        std::cout << "Failed to enable HID service on wiimote" << std::endl;
        return false;
    }

    return true;
}

bool forget_wiimote(BLUETOOTH_DEVICE_INFO& device_info) {
    if (!device_info.fConnected && device_info.fRemembered) {
        BluetoothRemoveDevice(&device_info.Address);
        return true;
    }

    return false;
}

template<typename T>
void enumerate_radios(T callback) {
    BLUETOOTH_FIND_RADIO_PARAMS radio_param;
    radio_param.dwSize = sizeof(radio_param);

    HANDLE radio;
    if (HBLUETOOTH_RADIO_FIND radio_find = BluetoothFindFirstRadio(&radio_param, &radio)) {
        do {
            BLUETOOTH_RADIO_INFO radio_info = {};
            radio_info.dwSize = sizeof(radio_info);

            if (BluetoothGetRadioInfo(radio, &radio_info) == ERROR_SUCCESS) {
                callback(radio, radio_info);
            }
            CloseHandle(radio);
        } while (BluetoothFindNextRadio(radio_find, &radio));

        BluetoothFindRadioClose(radio_find);
    }
}

template<typename T>
void enumerate_bluetooth_devices(BLUETOOTH_DEVICE_SEARCH_PARAMS search, T callback) {
    enumerate_radios([&](HANDLE radio, const BLUETOOTH_RADIO_INFO& radio_info) {
        search.hRadio = radio;

        BLUETOOTH_DEVICE_INFO device_info = {};
        device_info.dwSize = sizeof(device_info);

        if (HBLUETOOTH_DEVICE_FIND device_find = BluetoothFindFirstDevice(&search, &device_info)) {
            do {
                callback(radio, radio_info, device_info);
            } while (BluetoothFindNextDevice(device_find, &device_info));

            BluetoothFindDeviceClose(device_find);
        }
    });
}

void enable_wiimotes_hid_service() {
    BLUETOOTH_DEVICE_SEARCH_PARAMS search;
    search.dwSize = sizeof(search);
    search.fReturnAuthenticated = true;
    search.fReturnRemembered = true;
    search.fReturnUnknown = true;
    search.fReturnConnected = true;
    search.fIssueInquiry = true;
    search.cTimeoutMultiplier = 2;

    enumerate_bluetooth_devices(search, [&](HANDLE radio, const BLUETOOTH_RADIO_INFO& radio_info, BLUETOOTH_DEVICE_INFO& device_info) {
        std::string name = from_wstring(device_info.szName);
        if (is_wiimote_device_name(name)) {
            forget_wiimote(device_info);
            attach_wiimote(radio, device_info);
        }
    });
}

#endif

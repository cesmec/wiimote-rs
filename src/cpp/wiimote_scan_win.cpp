#ifdef _WIN32

#include "wiimote_api.h"

#include <windows.h>

#include <BluetoothAPIs.h>

#include <iostream>
#include <string>

std::string from_wstring(const std::wstring& wstr) {
    if (wstr.empty()) {
        return "";
    }
    int result_size = WideCharToMultiByte(CP_UTF8, 0, wstr.c_str(), (int)wstr.size(), nullptr, 0,
        nullptr, nullptr);
    if (result_size <= 0) {
        return "";
    }

    std::string result(result_size, '\0');
    WideCharToMultiByte(CP_UTF8, 0, wstr.c_str(), (int)wstr.size(), result.data(), result_size,
        nullptr, nullptr);
    return result;
}

bool is_wiimote_device_name(const std::string& name) {
    return name == "Nintendo RVL-CNT-01" || name == "Nintendo RVL-CNT-01-TR";
}

void register_as_hid_device(HANDLE radio, BLUETOOTH_DEVICE_INFO& device_info) {
    if (!device_info.fConnected && device_info.fRemembered) {
        BluetoothRemoveDevice(&device_info.Address);
    }
    if (device_info.fConnected || device_info.fRemembered) {
        return;
    }

    DWORD result = BluetoothSetServiceState(radio, &device_info,
        &HumanInterfaceDeviceServiceClass_UUID, BLUETOOTH_SERVICE_ENABLE);

    if (FAILED(result)) {
        std::cerr << "Failed to register wiimote as interface device" << std::endl;
    }
}

template <typename T>
void enumerate_bluetooth_radios(T callback) {
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
    } else {
        std::cerr << "No bluetooth adapter found" << std::endl;
    }
}

template <typename T>
void enumerate_bluetooth_devices(BLUETOOTH_DEVICE_SEARCH_PARAMS search, T callback) {
    enumerate_bluetooth_radios([&](HANDLE radio, const BLUETOOTH_RADIO_INFO& radio_info) {
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

    enumerate_bluetooth_devices(search,
        [&](HANDLE radio, const BLUETOOTH_RADIO_INFO& radio_info,
            BLUETOOTH_DEVICE_INFO& device_info) {
            std::string name = from_wstring(device_info.szName);
            if (is_wiimote_device_name(name)) {
                register_as_hid_device(radio, device_info);
            }
        });
}

#endif

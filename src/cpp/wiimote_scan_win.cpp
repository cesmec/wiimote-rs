#ifdef _WIN32

#include "scope_guard.h"
#include "wiimote_api.h"
#include "wiimote_win.h"

#include <windows.h>

#include <BluetoothAPIs.h>
#include <cfgmgr32.h>
#include <hidsdi.h>
#include <tchar.h>

#include <iostream>
#include <optional>
#include <queue>
#include <string>
#include <vector>

struct DeviceInfo {
    uint16_t vendor_id;
    uint16_t product_id;
    std::string serial_number;
};

typedef std::basic_string<TCHAR> TString;

std::queue<Wiimote*> wiimotes;
std::vector<BLUETOOTH_DEVICE_INFO> connected_wiimotes;

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

void register_as_hid_device(HANDLE radio, BLUETOOTH_DEVICE_INFO& device_info) {
    if (!device_info.fConnected && device_info.fRemembered) {
        BluetoothRemoveDevice(&device_info.Address);
    }
    if (device_info.fConnected || device_info.fRemembered) {
        return;
    }

    DWORD result = BluetoothSetServiceState(radio, &device_info,
        &HumanInterfaceDeviceServiceClass_UUID, BLUETOOTH_SERVICE_ENABLE);

    if (SUCCEEDED(result)) {
        connected_wiimotes.push_back(device_info);
    } else {
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

HANDLE open_wiimote_device(const TString& device_path, DWORD access) {
    constexpr auto SHARE_READ_WRITE = FILE_SHARE_READ | FILE_SHARE_WRITE;
    return CreateFile(device_path.c_str(), access, SHARE_READ_WRITE, NULL, OPEN_EXISTING, 0, NULL);
}

std::optional<DeviceInfo> get_device_info(const TString& device_path) {
    HANDLE device_handle = open_wiimote_device(device_path, 0);
    if (device_handle == INVALID_HANDLE_VALUE) {
        return {};
    }
    auto guard = sg::make_scope_guard([&]() { CloseHandle(device_handle); });

    HIDD_ATTRIBUTES attrib = {};
    attrib.Size = sizeof(HIDD_ATTRIBUTES);
    WCHAR name_buffer[64];
    if (HidD_GetAttributes(device_handle, &attrib)
        && HidD_GetSerialNumberString(device_handle, name_buffer, sizeof(name_buffer))) {
        return std::optional<DeviceInfo>(
            { attrib.VendorID, attrib.ProductID, from_wstring(name_buffer) });
    }
    return {};
}

template <typename T>
void enumerate_hid_devices(T callback) {
    GUID hid_id;
    HidD_GetHidGuid(&hid_id);

    DWORD length;
    CONFIGRET config_ret = CM_Get_Device_Interface_List_Size(&length, &hid_id, NULL,
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT);
    if (config_ret != CR_SUCCESS) {
        std::cerr << "Failed to get HID device list size" << std::endl;
        return;
    }

    TCHAR* device_list = new TCHAR[length];
    auto guard = sg::make_scope_guard([&]() { delete[] device_list; });

    config_ret = CM_Get_Device_Interface_List(&hid_id, NULL, device_list, length,
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT);
    if (config_ret != CR_SUCCESS) {
        std::cerr << "Failed to get HID device list" << std::endl;
        return;
    }

    for (TCHAR* device_path = device_list; device_path[0];
         device_path += _tcslen(device_path) + 1) {
        TString device_path_str = device_path;

        if (std::optional<DeviceInfo> device_info = get_device_info(device_path_str)) {
            callback(*device_info, device_path_str);
        }
    }
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

uint32_t wiimotes_scan() {
    enable_wiimotes_hid_service();

    enumerate_hid_devices([&](DeviceInfo device_info, const TString& device_path) {
        if (is_wiimote(device_info.vendor_id, device_info.product_id)) {
            HANDLE wiimote_handle = open_wiimote_device(device_path, GENERIC_READ | GENERIC_WRITE);
            if (wiimote_handle == INVALID_HANDLE_VALUE) {
                std::cerr << "Failed to connect to wiimote" << std::endl;
                return;
            }

            wiimotes.push(new Wiimote(device_info.serial_number, wiimote_handle));
        }
    });

    return wiimotes.size();
}

Wiimote* wiimotes_get_next() {
    if (wiimotes.empty()) {
        return nullptr;
    }

    Wiimote* wiimote = wiimotes.front();
    wiimotes.pop();
    return wiimote;
}

void wiimotes_scan_cleanup() {
    while (auto* wiimote = wiimotes_get_next()) {
        wiimote_cleanup(wiimote);
    }

    enumerate_bluetooth_radios([](HANDLE radio, const BLUETOOTH_RADIO_INFO& radio_info) {
        for (auto& connected_wiimote : connected_wiimotes) {
            BluetoothSetServiceState(radio, &connected_wiimote,
                &HumanInterfaceDeviceServiceClass_UUID, BLUETOOTH_SERVICE_DISABLE);
        }
    });

    connected_wiimotes.clear();
}

#endif

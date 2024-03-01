#pragma once

#include "wiimote_shared.h"

#include <windows.h>

#include <hidsdi.h>

#include <optional>
#include <vector>

class WiimoteWindows final : public WiimoteBase {
public:
    WiimoteWindows(const std::string& serial_number, HANDLE handle, HIDP_CAPS capabilities);
    ~WiimoteWindows();

    int32_t read(uint8_t* buffer, size_t buffer_size) final override;
    int32_t read_timeout(uint8_t* buffer, size_t buffer_size, size_t timeout_millis) final override;
    int32_t write(const uint8_t* buffer, size_t data_size) final override;

private:
    int32_t read_timeout_impl(uint8_t* buffer, size_t buffer_size,
        std::optional<size_t> timeout_millis);

private:
    HANDLE m_handle = 0;

    bool m_read_pending = false;
    bool m_write_pending = false;
    OVERLAPPED m_overlapped_read = {};
    OVERLAPPED m_overlapped_write = {};
    std::vector<uint8_t> m_read_buffer = {};
    std::vector<uint8_t> m_write_buffer = {};
};

using Wiimote = WiimoteWindows;

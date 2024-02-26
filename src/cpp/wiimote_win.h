#pragma once

#include "wiimote_shared.h"

#include <windows.h>

#include <optional>

class WiimoteWindows final : public WiimoteBase {
public:
    WiimoteWindows(const std::string& identifier, HANDLE handle);
    ~WiimoteWindows();

    int32_t read(uint8_t* buffer, size_t buffer_size) final override;
    int32_t read_timeout(uint8_t* buffer, size_t buffer_size, size_t timeout_millis) final override;
    int32_t write(const uint8_t* buffer, size_t data_size) final override;

private:
    int32_t read_timeout_impl(uint8_t* buffer, size_t buffer_size,
        std::optional<size_t> timeout_millis);

private:
    HANDLE m_handle = 0;
    OVERLAPPED m_overlapped = {};
    bool m_read_pending = false;
    uint8_t m_overlapped_read_buffer[32];
};

using Wiimote = WiimoteWindows;

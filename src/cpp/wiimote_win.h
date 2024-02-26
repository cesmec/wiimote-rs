#pragma once

#include "wiimote_shared.h"

#include <windows.h>

class WiimoteWindows final : public WiimoteBase {
public:
    WiimoteWindows(const std::string& identifier, HANDLE handle);
    ~WiimoteWindows();

    int32_t read(uint8_t* buffer, size_t buffer_size) final override;
    int32_t write(const uint8_t* buffer, size_t data_size) final override;

private:
    HANDLE m_handle = 0;
};

using Wiimote = WiimoteWindows;

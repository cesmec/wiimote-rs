#pragma once

#include "wiimote_shared.h"

class WiimoteLinux final : public WiimoteBase {
public:
    WiimoteLinux(const std::string& identifier, int control_socket, int data_socket);
    ~WiimoteLinux();

    int32_t read(uint8_t* buffer, size_t buffer_size) final override;
    int32_t write(const uint8_t* buffer, size_t data_size) final override;

private:
    int m_control_socket = 0;
    int m_data_socket = 0;
};

using Wiimote = WiimoteLinux;

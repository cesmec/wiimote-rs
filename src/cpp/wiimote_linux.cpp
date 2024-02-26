#ifndef _WIN32

#include "wiimote_linux.h"

#include "wiimote_api.h"

#include <assert.h>
#include <cstring>
#include <poll.h>
#include <sys/types.h>
#include <unistd.h>

WiimoteLinux::WiimoteLinux(const std::string& identifier, int control_socket, int data_socket)
    : WiimoteBase(identifier)
    , m_control_socket(control_socket)
    , m_data_socket(data_socket) { }

WiimoteLinux::~WiimoteLinux() {
    close(m_control_socket);
    close(m_data_socket);
}

// https://www.wiibrew.org/wiki/Wiimote#HID_Interface
// An "Input" report is sent by the Wii Remote to the host.
// An "Output" report is sent by the host to the Wii Remote.
// When using a Wii Remote, all input reports are prepended with 0xa1
// and all output reports are prepended with 0xa2 [...].
// Output reports are sent over the data pipe, which is also used to read input reports
// (thus, the control pipe is essentially unused).

constexpr const uint8_t INPUT_PREFIX = 0xA1;
constexpr const uint8_t OUTPUT_PREFIX = 0xA2;

int32_t WiimoteLinux::read(uint8_t* buffer, size_t buffer_size) {
    uint8_t read_buffer[DEFAULT_BUFFER_SIZE];

    size_t max_data_size = std::min(sizeof(read_buffer) - 1, buffer_size);
    ssize_t bytes_read = ::read(m_data_socket, read_buffer, max_data_size);
    if (bytes_read <= 0) {
        return bytes_read;
    }

    assert(read_buffer[0] == INPUT_PREFIX);
    memcpy(buffer, &read_buffer[1], bytes_read - 1);

    return static_cast<int32_t>(bytes_read - 1);
}

int32_t WiimoteLinux::read_timeout(uint8_t* buffer, size_t buffer_size, size_t timeout_millis) {
    pollfd read_poll;
    read_poll.fd = m_data_socket;
    read_poll.events = POLLIN;

    int result = poll(&read_poll, 1, timeout_millis);
    if (result <= 0) {
        return result;
    }

    return this->read(buffer, buffer_size);
}

int32_t WiimoteLinux::write(const uint8_t* buffer, size_t data_size) {
    uint8_t write_buffer[DEFAULT_BUFFER_SIZE];
    write_buffer[0] = OUTPUT_PREFIX;

    size_t data_bytes = std::min(sizeof(write_buffer) - 1, data_size);
    memcpy(&write_buffer[1], buffer, data_bytes);

    ssize_t bytes_written = ::write(m_data_socket, write_buffer, data_bytes + 1);
    return static_cast<int32_t>(bytes_written - 1);
}

#endif

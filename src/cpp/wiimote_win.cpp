#if _WIN32

#include "wiimote_win.h"

WiimoteWindows::WiimoteWindows(const std::string& identifier, HANDLE handle)
    : WiimoteBase(identifier)
    , m_handle(handle) { }

WiimoteWindows::~WiimoteWindows() {
    CloseHandle(m_handle);
}

int32_t WiimoteWindows::read(uint8_t* buffer, size_t buffer_size) {
    DWORD bytes_read = 0;
    if (!ReadFile(m_handle, buffer, buffer_size, &bytes_read, nullptr)) {
        return -1;
    }
    return static_cast<int32_t>(bytes_read);
}

int32_t WiimoteWindows::write(const uint8_t* buffer, size_t data_size) {
    DWORD bytes_written = 0;
    if (!WriteFile(m_handle, buffer, data_size, &bytes_written, nullptr)) {
        return -1;
    }
    return static_cast<int32_t>(bytes_written);
}

#endif

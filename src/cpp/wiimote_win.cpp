#if _WIN32

#include "wiimote_win.h"

#include "wiimote_api.h"

WiimoteWindows::WiimoteWindows(const std::string& identifier, HANDLE handle)
    : WiimoteBase(identifier)
    , m_handle(handle) {
    m_overlapped.hEvent = CreateEvent(NULL, false, false, NULL);

    static_assert(sizeof(m_overlapped_read_buffer) == DEFAULT_BUFFER_SIZE,
        "Wiimote read buffer size must match default buffer size");
}

WiimoteWindows::~WiimoteWindows() {
    CloseHandle(m_handle);
}

int32_t WiimoteWindows::read(uint8_t* buffer, size_t buffer_size) {
    return this->read_timeout_impl(buffer, buffer_size, {});
}

int32_t WiimoteWindows::read_timeout(uint8_t* buffer, size_t buffer_size, size_t timeout_millis) {
    return this->read_timeout_impl(buffer, buffer_size, timeout_millis);
}

int32_t WiimoteWindows::read_timeout_impl(uint8_t* buffer, size_t buffer_size,
    std::optional<size_t> timeout_millis) {
    buffer_size = std::min(buffer_size, sizeof(m_overlapped_read_buffer));
    DWORD bytes_read = 0;
    if (!m_read_pending) {
        ResetEvent(m_overlapped.hEvent);
        if (ReadFile(m_handle, m_overlapped_read_buffer, buffer_size, &bytes_read, &m_overlapped)) {
            memcpy(buffer, m_overlapped_read_buffer, bytes_read);
            return static_cast<int32_t>(bytes_read);
        }
        if (GetLastError() != ERROR_IO_PENDING) {
            return -1;
        }
        m_read_pending = true;
    }

    if (timeout_millis.has_value()) {
        DWORD wait_result = WaitForSingleObject(m_overlapped.hEvent, *timeout_millis);
        if (wait_result == WAIT_TIMEOUT) {
            return 0;
        }
        if (wait_result != WAIT_OBJECT_0) {
            // Wait failed
            return -1;
        }
    }

    m_read_pending = false;
    if (GetOverlappedResult(m_handle, &m_overlapped, &bytes_read, true)) {
        memcpy(buffer, m_overlapped_read_buffer, bytes_read);
        return static_cast<int32_t>(bytes_read);
    }
    return -1;
}

int32_t WiimoteWindows::write(const uint8_t* buffer, size_t data_size) {
    DWORD bytes_written = 0;
    if (!WriteFile(m_handle, buffer, data_size, &bytes_written, nullptr)) {
        return -1;
    }
    return static_cast<int32_t>(bytes_written);
}

#endif

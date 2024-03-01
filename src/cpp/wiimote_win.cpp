#if _WIN32

#include "wiimote_win.h"

// Implemented in wiimore_scan_win.cpp
void wiimote_disconnected(const std::string& serial_number);

WiimoteWindows::WiimoteWindows(const std::string& serial_number, HANDLE handle,
    HIDP_CAPS capabilities)
    : WiimoteBase(serial_number)
    , m_handle(handle) {
    m_overlapped_read.hEvent = CreateEvent(NULL, TRUE, FALSE, NULL);
    m_overlapped_write.hEvent = CreateEvent(NULL, TRUE, FALSE, NULL);

    m_read_buffer.resize(capabilities.InputReportByteLength);
    m_write_buffer.resize(capabilities.OutputReportByteLength);
}

WiimoteWindows::~WiimoteWindows() {
    CloseHandle(m_overlapped_read.hEvent);
    CloseHandle(m_overlapped_write.hEvent);
    CloseHandle(m_handle);

    wiimote_disconnected(get_identifier());
}

int32_t WiimoteWindows::read(uint8_t* buffer, size_t buffer_size) {
    return this->read_timeout_impl(buffer, buffer_size, {});
}

int32_t WiimoteWindows::read_timeout(uint8_t* buffer, size_t buffer_size, size_t timeout_millis) {
    return this->read_timeout_impl(buffer, buffer_size, timeout_millis);
}

int32_t WiimoteWindows::read_timeout_impl(uint8_t* buffer, size_t buffer_size,
    std::optional<size_t> timeout_millis) {
    buffer_size = std::min(buffer_size, m_read_buffer.size());
    bool did_read = false;
    if (!m_read_pending) {
        ResetEvent(m_overlapped_read.hEvent);
        std::fill(m_read_buffer.begin(), m_read_buffer.end(), 0);
        did_read = ReadFile(m_handle, m_read_buffer.data(), m_read_buffer.size(), nullptr,
            &m_overlapped_read);
        if (!did_read && GetLastError() != ERROR_IO_PENDING) {
            return -1;
        }

        m_read_pending = true;
    }

    if (!did_read && timeout_millis.has_value()) {
        DWORD wait_result = WaitForSingleObject(m_overlapped_read.hEvent, *timeout_millis);
        if (wait_result == WAIT_TIMEOUT) {
            return 0;
        }
        if (wait_result != WAIT_OBJECT_0) {
            // Wait failed
            return -1;
        }
    }

    DWORD bytes_read = 0;
    bool result = GetOverlappedResult(m_handle, &m_overlapped_read, &bytes_read, true);
    m_read_pending = false;
    if (result) {
        size_t bytes_to_copy = std::min(static_cast<size_t>(bytes_read), buffer_size);
        std::copy_n(m_read_buffer.cbegin(), bytes_to_copy, buffer);
        return static_cast<int32_t>(bytes_to_copy);
    }
    return -1;
}

int32_t WiimoteWindows::write(const uint8_t* buffer, size_t data_size) {
    if (m_write_pending) {
        WaitForSingleObject(m_overlapped_write.hEvent, INFINITE);
    }
    m_write_pending = true;
    data_size = std::min(data_size, m_write_buffer.size());
    std::copy_n(buffer, data_size, m_write_buffer.begin());
    std::fill(m_write_buffer.begin() + data_size, m_write_buffer.end(), 0);

    if (!WriteFile(m_handle, m_write_buffer.data(), m_write_buffer.size(), nullptr,
            &m_overlapped_write)) {
        if (GetLastError() != ERROR_IO_PENDING) {
            return -1;
        }

        DWORD wait_result = WaitForSingleObject(m_overlapped_write.hEvent, INFINITE);
        if (wait_result != WAIT_OBJECT_0) {
            m_write_pending = false;
            return -1;
        }
    }

    m_write_pending = false;
    DWORD bytes_written = 0;
    if (!GetOverlappedResult(m_handle, &m_overlapped_write, &bytes_written, true)) {
        return -1;
    }
    return static_cast<int32_t>(bytes_written);
}

#endif

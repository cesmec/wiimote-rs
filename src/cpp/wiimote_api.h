#pragma once

#include <cstddef>
#include <cstdint>

#if _WIN32
#include "wiimote_win.h"
#else
#include "wiimote_linux.h"
#endif

extern "C" {
    /** Deprecated */
    void enable_wiimotes_hid_service();
    constexpr const size_t DEFAULT_BUFFER_SIZE = 32;
    /**
     * Scan for available wiimotes.
     * Returns the number of connected wiimotes.
     */
    uint32_t wiimotes_scan();
    /**
     * Get the next wiimote from the connected devices queue.
     * Ownership is transferred to the caller and the resource can be cleaned up
     * using `wiimote_cleanup`.
     */
    Wiimote* wiimotes_get_next();
    /**
     * Cleanup resources used for wiimote scanning and disconnects all connected wiimotes.
     */
    void wiimotes_scan_cleanup();

    /**
     * Read n bytes from the wiimote.
     * Returns the number of bytes read, 0 on EOF or -1 on error.
     */
    int32_t wiimote_read(Wiimote* wiimote, uint8_t* buffer, size_t buffer_size);
    /**
     * Write n bytes to the wiimote.
     * Returns the number of bytes written or -1 on error.
     */
    int32_t wiimote_write(Wiimote* wiimote, const uint8_t* buffer, size_t data_size);
    /**
     * Cleanup resources when the wiimote connection is no longer needed.
     */
    void wiimote_cleanup(Wiimote* wiimote);
}

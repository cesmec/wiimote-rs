#pragma once

#include <cstddef>
#include <cstdint>
#include <string>

class WiimoteBase {
public:
    virtual int32_t read(uint8_t* buffer, size_t buffer_size) = 0;
    virtual int32_t write(const uint8_t* buffer, size_t data_size) = 0;
};

constexpr uint16_t WIIMOTE_VENDOR_ID = 0x057E;
constexpr uint16_t WIIMOTE_PRODUCT_ID = 0x0306;
constexpr uint16_t WIIMOTE_PLUS_PRODUCT_ID = 0x0330;

inline bool is_wiimote_device_name(const std::string& name) {
    return name == "Nintendo RVL-CNT-01" || name == "Nintendo RVL-CNT-01-TR";
}

inline bool is_wiimote(uint16_t vendor_id, uint16_t product_id) {
    return vendor_id == WIIMOTE_VENDOR_ID
        && (product_id == WIIMOTE_PRODUCT_ID || product_id == WIIMOTE_PLUS_PRODUCT_ID);
}

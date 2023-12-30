#pragma once

constexpr int WIIMOTE_VENDOR_ID = 0x057E;
constexpr int WIIMOTE_PRODUCT_ID = 0x0306;
constexpr int WIIMOTE_PLUS_PRODUCT_ID = 0x0330;

extern "C" {
    void enable_wiimotes_hid_service();
}

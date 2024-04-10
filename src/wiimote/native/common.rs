const WIIMOTE_VENDOR_ID: u16 = 0x057E;
const WIIMOTE_PRODUCT_ID: u16 = 0x0306;
const WIIMOTE_PLUS_PRODUCT_ID: u16 = 0x0330;

pub(super) const fn is_wiimote(vendor_id: u16, product_id: u16) -> bool {
    vendor_id == WIIMOTE_VENDOR_ID
        && (product_id == WIIMOTE_PRODUCT_ID || product_id == WIIMOTE_PLUS_PRODUCT_ID)
}

pub(super) fn is_wiimote_device_name(name: &str) -> bool {
    name == "Nintendo RVL-CNT-01" || name == "Nintendo RVL-CNT-01-TR"
}

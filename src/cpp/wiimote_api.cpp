#include "wiimote_api.h"

int32_t wiimote_read(Wiimote* wiimote, uint8_t* buffer, size_t buffer_size) {
    return wiimote->read(buffer, buffer_size);
}

int32_t wiimote_write(Wiimote* wiimote, const uint8_t* buffer, size_t data_size) {
    return wiimote->write(buffer, data_size);
}

void wiimote_cleanup(Wiimote* wiimote) {
    delete wiimote;
}

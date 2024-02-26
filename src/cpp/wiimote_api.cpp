#include "wiimote_api.h"

#include <cstring>

int32_t wiimote_read(Wiimote* wiimote, uint8_t* buffer, size_t buffer_size) {
    return wiimote->read(buffer, buffer_size);
}

int32_t wiimote_write(Wiimote* wiimote, const uint8_t* buffer, size_t data_size) {
    return wiimote->write(buffer, data_size);
}

size_t wiimote_get_identifier_length(Wiimote* wiimote) {
    return wiimote->get_identifier().length() + 1;
}

bool wiimote_get_identifier(Wiimote* wiimote, char* identifier, size_t identifier_buffer_length) {
    const std::string& wiimote_identifier = wiimote->get_identifier();
    size_t length_with_null = wiimote_identifier.length() + 1;
    if (length_with_null > identifier_buffer_length) {
        return false;
    }

    memcpy(identifier, wiimote_identifier.c_str(), length_with_null);
    return true;
}

void wiimote_cleanup(Wiimote* wiimote) {
    delete wiimote;
}

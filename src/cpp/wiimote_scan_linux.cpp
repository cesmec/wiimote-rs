#ifndef _WIN32

#include "scope_guard.h"
#include "wiimote_api.h"
#include "wiimote_linux.h"

#include <bluetooth/bluetooth.h>
#include <bluetooth/hci.h>
#include <bluetooth/hci_lib.h>
#include <bluetooth/l2cap.h>
#include <sys/socket.h>
#include <unistd.h>

#include <cstdio>
#include <optional>
#include <queue>

constexpr uint16_t CONTROL_PIPE_ID = 0x0011;
constexpr uint16_t DATA_PIPE_ID = 0x0013;

std::queue<Wiimote*> wiimotes;

std::optional<int> connect_socket(sockaddr_l2 addr) {
    int socket_fd = socket(AF_BLUETOOTH, SOCK_SEQPACKET, BTPROTO_L2CAP);
    if (socket_fd < 0) {
        perror("Unable to open socket to Wiimote");
        return {};
    }

    if (connect(socket_fd, (sockaddr*)&addr, sizeof(addr)) < 0) {
        perror("Unable to connect channel of Wiimote");
        close(socket_fd);
        return {};
    }
    return socket_fd;
}

void handle_wiimote(bdaddr_t bdaddr) {
    sockaddr_l2 addr = {};
    addr.l2_family = AF_BLUETOOTH;
    addr.l2_bdaddr = bdaddr;

    addr.l2_psm = htobs(CONTROL_PIPE_ID);
    std::optional<int> control_socket = connect_socket(addr);
    if (!control_socket) {
        return;
    }

    addr.l2_psm = htobs(DATA_PIPE_ID);
    std::optional<int> data_socket = connect_socket(addr);
    if (!data_socket) {
        close(*control_socket);
        return;
    }

    char address_string[19] = { 0 };
    ba2str(&bdaddr, address_string);

    wiimotes.push(new Wiimote(address_string, *control_socket, *data_socket));
}

uint32_t wiimotes_scan() {
    constexpr int MAX_INQUIRIES = 255;
    inquiry_info infos[MAX_INQUIRIES] {};
    inquiry_info* info_ptr = infos;

    int bt_device_id = hci_get_route(NULL);
    int bt_socket = hci_open_dev(bt_device_id);
    if (bt_device_id < 0 || bt_socket < 0) {
        perror("Failed to open default bluetooth device");
        return wiimotes.size();
    }
    auto guard = sg::make_scope_guard([&]() { close(bt_socket); });

    constexpr int SCAN_SECONDS = 8;
    int flags = IREQ_CACHE_FLUSH;

    int device_count
        = hci_inquiry(bt_device_id, SCAN_SECONDS, MAX_INQUIRIES, NULL, &info_ptr, flags);
    if (device_count < 0) {
        perror("hci_inquiry failed while scanning for bluetooth devices");
        return wiimotes.size();
    }

    for (int i = 0; i < device_count; i++) {
        char name[250] = {};

        if (hci_read_remote_name(bt_socket, &infos[i].bdaddr, sizeof(name), name, 0) < 0) {
            continue;
        }

        if (is_wiimote_device_name(name)) {
            handle_wiimote(infos[i].bdaddr);
        }
    }

    return wiimotes.size();
}

Wiimote* wiimotes_get_next() {
    if (wiimotes.empty()) {
        return nullptr;
    }

    Wiimote* wiimote = wiimotes.front();
    wiimotes.pop();
    return wiimote;
}

void wiimotes_scan_cleanup() {
    while (auto* wiimote = wiimotes_get_next()) {
        wiimote_cleanup(wiimote);
    }
}

#endif

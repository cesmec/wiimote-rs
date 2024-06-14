mod bindings;

use std::ffi::c_int;

use nix::libc::{
    connect, poll, pollfd, sockaddr, socket, write, AF_BLUETOOTH, POLLIN, SOCK_SEQPACKET,
};
use nix::unistd::{close, read};

use crate::WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE;

use self::bindings::{
    ba2str, bdaddr_t, hci_get_route, hci_inquiry, hci_open_dev, hci_read_remote_name, inquiry_info,
    sockaddr_l2, BTPROTO_L2CAP, IREQ_CACHE_FLUSH,
};

use super::common::is_wiimote_device_name;
use super::NativeWiimote;

const MAX_INQUIRIES: i32 = 255;
const SCAN_SECONDS: i32 = 6;
const MAX_NAME_LENGTH: i32 = 250;

const CONTROL_PIPE_ID: u16 = 0x0011;
const DATA_PIPE_ID: u16 = 0x0013;

unsafe fn connect_socket(address: sockaddr_l2) -> Option<c_int> {
    let socket_fd = socket(AF_BLUETOOTH as _, SOCK_SEQPACKET as _, BTPROTO_L2CAP as _);
    if socket_fd < 0 {
        eprintln!("Unable to open socket to Wiimote");
        return None;
    }

    let address_ptr = std::ptr::addr_of!(address).cast::<sockaddr>();
    let address_size = std::mem::size_of_val(&address);
    if connect(socket_fd, address_ptr, address_size as _) < 0 {
        eprintln!("Unable to connect channel of Wiimote");
        _ = close(socket_fd);
        return None;
    }
    Some(socket_fd)
}

unsafe fn handle_wiimote(bdaddr: bdaddr_t) -> Option<LinuxNativeWiimote> {
    let mut addr = std::mem::zeroed::<sockaddr_l2>();
    addr.l2_family = AF_BLUETOOTH as _;
    addr.l2_bdaddr = bdaddr;

    addr.l2_psm = CONTROL_PIPE_ID;
    let control_socket = connect_socket(addr)?;

    addr.l2_psm = DATA_PIPE_ID;
    let data_socket = connect_socket(addr);
    if data_socket.is_none() {
        _ = close(control_socket);
        return None;
    }

    let mut address_string = [0u8; 19];
    ba2str(&bdaddr, address_string.as_mut_ptr().cast());

    let address = String::from_utf8_lossy(&address_string);
    Some(LinuxNativeWiimote::new(
        &address,
        control_socket,
        data_socket.unwrap(),
    ))
}

pub fn wiimotes_scan(wiimotes: &mut Vec<LinuxNativeWiimote>) {
    unsafe {
        let mut infos = Vec::with_capacity(MAX_INQUIRIES as _);
        for _ in 0..MAX_INQUIRIES {
            infos.push(std::mem::zeroed::<inquiry_info>());
        }

        let bt_device_id = hci_get_route(std::ptr::null_mut());
        let bt_socket = hci_open_dev(bt_device_id);
        if bt_device_id < 0 || bt_socket < 0 {
            eprintln!("Failed to open default bluetooth device");
            return;
        }

        let device_count = hci_inquiry(
            bt_device_id,
            SCAN_SECONDS,
            MAX_INQUIRIES,
            std::ptr::null(),
            &mut infos.as_mut_ptr(),
            IREQ_CACHE_FLUSH as _,
        );
        if device_count < 0 {
            _ = close(bt_socket);
            eprintln!("hci_inquiry failed while scanning for bluetooth devices");
            return;
        }

        for info in infos.iter().take(device_count as _) {
            let mut name = [0u8; (MAX_NAME_LENGTH + 1) as _];

            if hci_read_remote_name(
                bt_socket,
                &info.bdaddr,
                MAX_NAME_LENGTH,
                name.as_mut_ptr().cast(),
                0,
            ) < 0
            {
                continue;
            }

            let name_length = name.iter().position(|&c| c == 0).unwrap();
            let name = String::from_utf8_lossy(&name[..name_length]);
            if is_wiimote_device_name(&name) {
                if let Some(wiimote) = handle_wiimote(info.bdaddr) {
                    wiimotes.push(wiimote);
                }
            }
        }

        _ = close(bt_socket);
    }
}

pub const fn wiimotes_scan_cleanup() {}

pub struct LinuxNativeWiimote {
    address: String,
    control_socket: c_int,
    data_socket: c_int,
}

impl LinuxNativeWiimote {
    fn new(address: &str, control_socket: c_int, data_socket: c_int) -> Self {
        Self {
            address: address.to_string(),
            control_socket,
            data_socket,
        }
    }
}

const INPUT_PREFIX: u8 = 0xA1;
const OUTPUT_PREFIX: u8 = 0xA2;

impl NativeWiimote for LinuxNativeWiimote {
    fn read(&mut self, buffer: &mut [u8]) -> Option<usize> {
        let mut read_buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];

        let max_data_size = usize::min(read_buffer.len() - 1, buffer.len());
        let bytes_read = read(self.data_socket, &mut read_buffer[..max_data_size]).ok()?;
        if bytes_read == 0 {
            return None;
        }

        debug_assert!(read_buffer[0] == INPUT_PREFIX);
        buffer[..bytes_read - 1].copy_from_slice(&read_buffer[1..bytes_read]);

        Some(bytes_read - 1)
    }

    fn read_timeout(&mut self, buffer: &mut [u8], timeout_millis: usize) -> Option<usize> {
        const TIMED_OUT: i32 = 0;
        let mut read_poll = unsafe { std::mem::zeroed::<pollfd>() };
        read_poll.fd = self.data_socket;
        read_poll.events = POLLIN;

        let mut fds = [read_poll];

        let result = unsafe { poll(fds.as_mut_ptr(), 1, timeout_millis as _) };
        if result == TIMED_OUT {
            return Some(0);
        }
        if result < 0 {
            return None;
        }

        self.read(buffer)
    }

    fn write(&mut self, buffer: &[u8]) -> Option<usize> {
        let mut write_buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];
        write_buffer[0] = OUTPUT_PREFIX;

        let data_bytes = usize::min(write_buffer.len() - 1, buffer.len());
        write_buffer[1..=data_bytes].copy_from_slice(&buffer[..data_bytes]);

        let bytes_written = unsafe {
            write(
                self.data_socket,
                write_buffer.as_ptr().cast(),
                data_bytes + 1,
            )
        };
        if bytes_written <= 0 {
            None
        } else {
            Some((bytes_written - 1) as _)
        }
    }

    fn identifier(&self) -> String {
        self.address.clone()
    }
}

impl Drop for LinuxNativeWiimote {
    fn drop(&mut self) {
        _ = close(self.control_socket);
        _ = close(self.data_socket);
    }
}

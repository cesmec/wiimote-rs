use std::ffi::c_void;

use wiimote_rs::prelude::InputReport;

extern crate wiimote_rs;

fn main() {
    unsafe {
        let wiimote_count = wiimotes_scan();

        for _ in 0..wiimote_count {
            let wiimote = wiimotes_get_next();
            handle_wiimote(wiimote);
            wiimote_cleanup(wiimote);
        }

        wiimotes_scan_cleanup();
    }
}

unsafe fn handle_wiimote(wiimote: *mut c_void) {
    // https://www.wiibrew.org/wiki/Wiimote#Player_LEDs
    const LED1: u8 = 0x10;
    const LED4: u8 = 0x80;

    println!("Found a wiimote! changing leds...");
    let data: [u8; 2] = [0x11, LED1 | LED4];
    wiimote_write(wiimote, data.as_ptr(), data.len());

    // https://www.wiibrew.org/wiki/Wiimote#Data_Reporting
    // Switch to reporting mode 31 (Core Buttons and Accelerometer)
    // let data: [u8; 3] = [0x12, 0x00, 0x31];
    // wiimote_write(wiimote, data.as_ptr(), data.len());

    // https://www.wiibrew.org/wiki/Wiimote/Extension_Controllers/Wii_Motion_Plus
    // Check for Wii Motion Plus
    let data: [u8; 7] = [0x17, 0x04, 0xa6, 0x00, 0xfa, 0x00, 0x06];
    wiimote_write(wiimote, data.as_ptr(), data.len());

    // https://www.wiibrew.org/wiki/Wiimote#0x20:_Status
    // Request status report
    // let data: [u8; 2] = [0x15, 0x00];
    // wiimote_write(wiimote, data.as_ptr(), data.len());

    let mut result_buffer = [0u8; 32];
    loop {
        let read_bytes = wiimote_read(wiimote, result_buffer.as_mut_ptr(), result_buffer.len());
        if read_bytes < 0 {
            eprintln!("Read failed");
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        let result_data = &result_buffer[..(read_bytes as usize)];
        let result = InputReport::try_from(result_buffer).expect("Failed to convert input report");
        println!("Result: {result_data:X?} {result:?}");
        if let InputReport::DataReport(_, report_data) = result {
            const HOME_BUTTON: u8 = 0x80;
            if report_data.data[1] & HOME_BUTTON != 0 {
                println!("Home pressed");
                break;
            }
        }
    }
}

extern "C" {
    fn wiimotes_scan() -> u32;
    fn wiimotes_get_next() -> *mut c_void;
    fn wiimotes_scan_cleanup();

    fn wiimote_read(wiimote: *mut c_void, buffer: *mut u8, buffer_size: usize) -> i32;
    fn wiimote_write(wiimote: *mut c_void, buffer: *const u8, data_size: usize) -> i32;
    fn wiimote_cleanup(wiimote: *mut c_void);
}

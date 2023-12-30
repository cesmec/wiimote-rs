use wiimote_rs::prelude::*;

fn main() {
    unsafe {
        enable_wiimotes_hid_service();
    }

    let hid_api = HidApi::new().unwrap();
    hid_api.device_list().for_each(|device_info| {
        let wiimote = WiimoteDevice::new(device_info, &hid_api);
        match wiimote {
            Ok(wiimote) => handle_wiimote(&wiimote),
            Err(_err) => println!(
                "Other device: {device_info:?} ({} - {})",
                device_info.manufacturer_string().unwrap_or("Unknown"),
                device_info.product_string().unwrap_or("Unknown")
            ),
        }
    });
}

fn handle_wiimote(wiimote: &WiimoteDevice) {
    // https://www.wiibrew.org/wiki/Wiimote#Player_LEDs
    const LED1: u8 = 0x10;
    const LED4: u8 = 0x80;

    println!("Found a wiimote! changing leds...");
    let data: [u8; 2] = [0x11, LED1 | LED4];
    wiimote.write(&data).unwrap();

    // https://www.wiibrew.org/wiki/Wiimote#Data_Reporting
    // Switch to reporting mode 31 (Core Buttons and Accelerometer)
    // let data: [u8; 3] = [0x12, 0x00, 0x31];
    // wiimote.write(&data).unwrap();

    // https://www.wiibrew.org/wiki/Wiimote/Extension_Controllers/Wii_Motion_Plus
    // Check for Wii Motion Plus
    let data: [u8; 7] = [0x17, 0x04, 0xa6, 0x00, 0xfa, 0x00, 0x06];
    wiimote.write(&data).unwrap();

    // https://www.wiibrew.org/wiki/Wiimote#0x20:_Status
    // Request status report
    // let data: [u8; 2] = [0x15, 0x00];
    // wiimote.write(&data).unwrap();

    let mut result_buffer = [0u8; 32];
    loop {
        let read_bytes = wiimote.read(&mut result_buffer).unwrap();

        let result_data = &result_buffer[..read_bytes];
        println!("Result: {result_data:?}");
        match result_data[0] {
            0x20 => {
                println!("status");
            }
            0x21 => {
                println!("memory and register data");
            }
            0x22 => {
                println!("ack / return");
            }
            0x30..=0x3f => {
                println!("data report");
            }
            _ => {
                println!("Unknown result!");
            }
        }
    }
}

extern "C" {
    fn enable_wiimotes_hid_service();
}

use std::time::Duration;

use wiimote_rs::prelude::*;

fn main() -> WiimoteResult<()> {
    // Press the 1 and 2 buttons on the Wii remote to connect

    let (tx, rx) = std::sync::mpsc::channel();

    let _output = std::thread::spawn(move || {
        // Logs all reports from the connected Wii remotes
        while let Ok(message) = rx.recv() {
            dbg!(message);
        }
    });

    let manager = WiimoteManager::get_instance();

    let new_devices = {
        let manager = manager.lock().unwrap();
        manager.new_devices_receiver()
    };

    new_devices.iter().try_for_each(|d| -> WiimoteResult<()> {
        let tx = tx.clone();

        std::thread::spawn(move || {
            let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];

            let led_report = OutputReport::PlayerLed(PlayerLedFlags::LED_2 | PlayerLedFlags::LED_3);
            let size = led_report.fill_buffer(false, &mut buffer);
            d.lock().unwrap().write(&buffer[..size]).unwrap();

            std::thread::sleep(std::time::Duration::from_millis(1000));

            let led_report = OutputReport::PlayerLed(PlayerLedFlags::LED_1 | PlayerLedFlags::LED_4);
            let size = led_report.fill_buffer(false, &mut buffer);
            d.lock().unwrap().write(&buffer[..size]).unwrap();

            loop {
                let size = d.lock().unwrap().read_timeout(&mut buffer, 50).unwrap_or(0);
                if size > 0 {
                    let report = InputReport::try_from(buffer).unwrap();
                    tx.send(report).unwrap();
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        Ok(())
    })?;

    Ok(())
}

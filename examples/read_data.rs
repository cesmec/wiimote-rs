use std::time::Duration;

use wiimote_rs::output::{OutputReport, PlayerLedFlags};
use wiimote_rs::prelude::*;

fn main() -> WiimoteResult<()> {
    // Press the 1 and 2 buttons on the Wii remote to connect

    let manager = WiimoteManager::get_instance();

    let new_devices = {
        let manager = manager.lock().unwrap();
        manager.new_devices_receiver()
    };

    new_devices.iter().try_for_each(|d| -> WiimoteResult<()> {
        std::thread::spawn(move || {
            let led_report = OutputReport::PlayerLed(PlayerLedFlags::LED_2 | PlayerLedFlags::LED_3);
            d.lock().unwrap().write(&led_report).unwrap();

            std::thread::sleep(std::time::Duration::from_millis(1000));

            let led_report = OutputReport::PlayerLed(PlayerLedFlags::LED_1 | PlayerLedFlags::LED_4);
            d.lock().unwrap().write(&led_report).unwrap();

            loop {
                let input_report = d.lock().unwrap().read_timeout(50);
                if let Ok(report) = input_report {
                    dbg!(report);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        Ok(())
    })?;

    Ok(())
}

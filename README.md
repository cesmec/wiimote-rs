# wiimote-rs

[![crates.io](https://img.shields.io/crates/v/wiimote-rs.svg)](https://crates.io/crates/wiimote-rs)
[![CI](https://github.com/cesmec/wiimote-rs/workflows/CI/badge.svg)](https://github.com/cesmec/wiimote-rs/actions)
[![Documentation](https://docs.rs/wiimote-rs/badge.svg)](https://docs.rs/wiimote-rs)
![License](https://img.shields.io/crates/l/wiimote-rs.svg)

A Rust library to communicate with Wii remotes over Bluetooth.

## Features

`wiimote-rs` is in development and currently supports:

- Connect Wii remotes over Bluetooth by pressing the `1`+`2` buttons
- Send data as output reports
- Receive data as input reports
- Read accelerometer calibration and convert from raw values
- Read motion plus calibration and convert from raw values
- Read balance board calibration and convert from raw values

## Setup

Windows: no additional setup required

Linux: install the following packages on debian-based systems, or their equivalent on other distributions:

```bash
sudo apt install libudev-dev libbluetooth-dev clang
```

macOS: not supported at the moment

## Examples

Check the `examples` directory for full examples.

### Accept Wii remote connections

```rust
use wiimote_rs::prelude::*;

fn main() -> WiimoteResult<()> {
    let manager = WiimoteManager::get_instance();
    let new_devices = {
        let manager = manager.lock().unwrap();
        manager.new_devices_receiver()
    };

    new_devices.iter().try_for_each(|device| -> WiimoteResult<()> {
        // Do something with the connected Wii remote
        Ok(())
    })
}
```

### Send data to Wii remotes

```rust
use std::sync::{Arc, Mutex};

use wiimote_rs::prelude::*;

use wiimote_rs::output::{OutputReport, PlayerLedFlags};

fn change_leds(device: Arc<Mutex<WiimoteDevice>>) -> WiimoteResult<()> {
    let led_report = OutputReport::PlayerLed(PlayerLedFlags::LED_2 | PlayerLedFlags::LED_3);
    device.lock().unwrap().write(&led_report)
}
```

### Receive data from Wii remotes

```rust
use std::sync::{Arc, Mutex};

use wiimote_rs::prelude::*;

use wiimote_rs::input::InputReport;

fn read_buttons(device: Arc<Mutex<WiimoteDevice>>) -> WiimoteResult<()> {
    let input_report = device.lock().unwrap().read()?;
    match input_report {
        InputReport::DataReport(_, data) => {
            // All data reports except 0x3d contain button data
            let buttons = data.buttons();
        }
        _ => {}
    }
    Ok(())
}
```

### Read accelerometer data

```rust
use std::sync::{Arc, Mutex};

use wiimote_rs::prelude::*;

use wiimote_rs::input::InputReport;

fn read_accelerometer(device: Arc<Mutex<WiimoteDevice>>) -> WiimoteResult<()> {
    // The accelerometer calibration can be stored and reused per WiimoteDevice
    let accelerometer_calibration = device.lock().unwrap().accelerometer_calibration().unwrap().clone();

    let input_report = device.lock().unwrap().read()?;
    match input_report {
        // Note that the data report mode needs to be set to a mode that includes accelerometer data such as 0x31
        InputReport::DataReport(0x31, wiimote_data) => {
            let accelerometer_data = AccelerometerData::from_normal_reporting(&wiimote_data.data);
            let (x, y, z) = accelerometer_calibration.get_acceleration(&accelerometer_data);
        }
        _ => {}
    }
    Ok(())
}
```

#![allow(clippy::option_if_let_else)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use wiimote_rs::input::InputReport;
use wiimote_rs::output::{DataReporingMode, OutputReport, PlayerLedFlags};
use wiimote_rs::prelude::*;

fn main() -> WiimoteResult<()> {
    // Press the 1 and 2 buttons on the Wii remote to connect

    let (tx, rx) = std::sync::mpsc::channel();

    let _output = std::thread::spawn(move || {
        // Logs all reports from the connected Wii remotes
        while let Ok(_message) = rx.recv() {
            // dbg!(message);
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

            let (accelerometer_calibration, motion_plus_calibration) = {
                let wiimote = d.lock().unwrap();
                if let Some(motion_plus) = wiimote.motion_plus() {
                    motion_plus.initialize(&wiimote).unwrap();
                    motion_plus
                        .change_mode(&wiimote, MotionPlusMode::Active)
                        .unwrap();
                }
                println!("Motion plus: {:?}", wiimote.motion_plus());
                println!("Extension: {:?}", wiimote.extension());
                (
                    wiimote.accelerometer_calibration().clone(),
                    wiimote.motion_plus().map(MotionPlus::calibration),
                )
            };

            set_reporting_mode_accelerometer_and_extension(&d);

            loop {
                let size = d.lock().unwrap().read_timeout(&mut buffer, 50).unwrap_or(0);
                if size > 0 {
                    let report = InputReport::try_from(buffer).unwrap();
                    handle_report(
                        &report,
                        &accelerometer_calibration,
                        &motion_plus_calibration,
                        &d,
                    );
                    tx.send(report).unwrap();
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        Ok(())
    })?;

    Ok(())
}

fn handle_report(
    report: &InputReport,
    accelerometer_calibration: &AccelerometerCalibration,
    motion_plus_calibration: &Option<MotionPlusCalibration>,
    d: &Arc<Mutex<WiimoteDevice>>,
) {
    if let InputReport::StatusInformation(_) = report {
        // If this report is received when not requested, the application 'MUST'
        // send report 0x12 to change the data reporting mode, otherwise no further data reports will be received.
        set_reporting_mode_accelerometer_and_extension(d);
    } else if let InputReport::DataReport(0x35, wiimote_data) = &report {
        if let Some(calibration) = &motion_plus_calibration {
            let accelerometer_data = AccelerometerData::from_normal_reporting(&wiimote_data.data);
            let (x, y, z) = accelerometer_calibration.get_acceleration(&accelerometer_data);

            let mut motion_plus_buffer = [0u8; 6];
            motion_plus_buffer.copy_from_slice(&wiimote_data.data[5..11]);

            if let Ok(motion_plus_data) = MotionPlusData::try_from(motion_plus_buffer) {
                let (yaw, roll, pitch) = calibration.get_angular_velocity(&motion_plus_data);
                print!("\rX: {x}, Y: {y}, Z: {z} | Yaw: {yaw}, Roll: {roll}, Pitch: {pitch}               ");
            } else {
                print!("\rX: {x}, Y: {y}, Z: {z} | Motion plus data error                                 ");
            }
        }
    }
}

fn set_reporting_mode_accelerometer_and_extension(d: &Arc<Mutex<WiimoteDevice>>) {
    let mut buffer = [0u8; WIIMOTE_DEFAULT_REPORT_BUFFER_SIZE];

    let reporting_mode = OutputReport::DataReportingMode(DataReporingMode {
        continuous: false,
        mode: 0x35, // Core Buttons and Accelerometer with 16 Extension Bytes
    });
    let size = reporting_mode.fill_buffer(false, &mut buffer);
    d.lock().unwrap().write(&buffer[..size]).unwrap();
}

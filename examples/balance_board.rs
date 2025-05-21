use std::sync::{Arc, Mutex};
use std::time::Duration;

use wiimote_rs::extensions::{BalanceBoardCalibration, BalanceBoardData, WiimoteExtension};
use wiimote_rs::input::InputReport;
use wiimote_rs::output::{DataReporingMode, OutputReport};
use wiimote_rs::prelude::*;

fn main() -> WiimoteResult<()> {
    // Press the sync button on the balance board to connect

    let manager = WiimoteManager::get_instance();

    let new_devices = {
        let manager = manager.lock().unwrap();
        manager.new_devices_receiver()
    };

    new_devices.iter().try_for_each(|d| -> WiimoteResult<()> {
        std::thread::spawn(move || {
            let calibration = {
                let mut wiimote = d.lock().unwrap();
                if matches!(wiimote.extension(), Some(WiimoteExtension::BalanceBoard)) {
                    wiimote
                        .balance_board_calibration()
                        .expect("Failed to read balance board calibration data")
                        .clone()
                } else {
                    // Automatically disconnect if device is not a balance board
                    wiimote.disconnect();
                    return;
                }
            };

            set_reporting_mode_buttons_and_extension(&d);

            loop {
                let input_report = d.lock().unwrap().read_timeout(50);
                if let Ok(report) = input_report {
                    handle_report(&report, &calibration, &d);
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
    calibration: &BalanceBoardCalibration,
    d: &Arc<Mutex<WiimoteDevice>>,
) {
    if let InputReport::StatusInformation(_) = report {
        // If this report is received when not requested, the application 'MUST'
        // send report 0x12 to change the data reporting mode, otherwise no further data reports will be received.
        set_reporting_mode_buttons_and_extension(d);
    } else if let InputReport::DataReport(0x34, wiimote_data) = &report {
        if let Ok(balance_board_data) = BalanceBoardData::try_from(&wiimote_data.data[2..]) {
            let weights = calibration.get_weights(&balance_board_data);
            print!(
                "\rTotal: {:.2} Top left: {:.2} Top right: {:.2} Bottom left: {:.2} Bottom right: {:.2}          ",
                weights.total(),
                weights.top_left,
                weights.top_right,
                weights.bottom_left,
                weights.bottom_right
            );
        }
    }
}

fn set_reporting_mode_buttons_and_extension(d: &Arc<Mutex<WiimoteDevice>>) {
    // Reporting mode 0x32 Core Buttons with 8 Extension bytes could also be used for weight data excluding battery and temperature values.
    let reporting_mode = OutputReport::DataReportingMode(DataReporingMode {
        continuous: false,
        mode: 0x34, // Core Buttons with 19 Extension bytes
    });
    d.lock().unwrap().write(&reporting_mode).unwrap();
}

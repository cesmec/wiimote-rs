use crate::prelude::*;

use std::collections::{HashMap, HashSet};
use std::option::Option::Some;
use std::sync::{Arc, Mutex, Once};
use std::thread::JoinHandle;
use std::time::Duration;

use super::device::WiimoteDevice;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct WiimoteSerialNumber(pub String);

extern "C" {
    fn enable_wiimotes_hid_service();
}

type MutexWiimoteDevice = Arc<Mutex<WiimoteDevice>>;

struct ScanResult {
    new_devices: Vec<MutexWiimoteDevice>,
    reconnected_devices: Vec<MutexWiimoteDevice>,
}

/// Periodically checks for connections / disconnections of Wii remotes.
pub struct WiimoteManager {
    devices: HashMap<WiimoteSerialNumber, MutexWiimoteDevice>,
    hid_api: Option<HidApi>,
    scan_thread: Option<JoinHandle<()>>,
    scan_interval: Duration,
    new_devices_receiver: crossbeam_channel::Receiver<MutexWiimoteDevice>,
    reconnected_devices_receiver: crossbeam_channel::Receiver<MutexWiimoteDevice>,
}

impl WiimoteManager {
    /// Get the Wii remote manager instance.
    ///
    /// # Panics
    ///
    /// Panics if `HidApi` failes to initialize or detect devices.
    pub fn get_instance() -> Arc<Mutex<Self>> {
        static mut SINGLETON: Option<Arc<Mutex<WiimoteManager>>> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                let instance = Self::new().unwrap();

                SINGLETON = Some(instance);
            });

            SINGLETON.clone().unwrap_or_else(|| unreachable!())
        }
    }

    fn new() -> WiimoteResult<Arc<Mutex<Self>>> {
        Self::with_interval(Duration::from_millis(100))
    }

    fn with_interval(interval: Duration) -> WiimoteResult<Arc<Mutex<Self>>> {
        let (new_sender, new_devices_receiver) = crossbeam_channel::unbounded();
        let (reconnected_sender, reconnected_devices_receiver) = crossbeam_channel::unbounded();

        let manager = {
            let mut manager = Self {
                devices: HashMap::new(),
                hid_api: None,
                scan_thread: None,
                scan_interval: interval,
                new_devices_receiver,
                reconnected_devices_receiver,
            };

            // Immediately scan for devices
            let scan_result = manager.scan()?;
            for new_device in scan_result.new_devices {
                let _ = new_sender.send(new_device);
            }

            Arc::new(Mutex::new(manager))
        };

        let scan_thread = {
            let manager = Arc::downgrade(&manager);

            std::thread::spawn(move || {
                while let Some(manager) = manager.upgrade() {
                    let interval = {
                        let mut manager = match manager.lock() {
                            Ok(m) => m,
                            Err(m) => m.into_inner(),
                        };

                        if let Ok(ScanResult {
                            new_devices,
                            reconnected_devices,
                        }) = manager.scan()
                        {
                            let send_result = new_devices
                                .into_iter()
                                .try_for_each(|device| new_sender.send(device));
                            if send_result.is_err() {
                                // Channel is disconnected, end scan thread
                                return;
                            }

                            let send_result = reconnected_devices
                                .into_iter()
                                .try_for_each(|device| reconnected_sender.send(device));
                            if send_result.is_err() {
                                // Channel is disconnected, end scan thread
                                return;
                            }
                        }

                        manager.scan_interval
                    };

                    std::thread::sleep(interval);
                }
            })
        };

        if let Ok(mut manager) = manager.lock() {
            manager.scan_thread = Some(scan_thread);
        }

        Ok(manager)
    }

    pub fn set_scan_interval(&mut self, interval: Duration) {
        self.scan_interval = interval;
    }

    /// Scan the Wii remotes connected to your computer.
    ///
    /// # Errors
    ///
    /// Returns an error if `HidApi` failes to initialize or detect devices.
    fn scan(&mut self) -> WiimoteResult<ScanResult> {
        unsafe {
            // Make Wii remotes discoverable to HID API
            enable_wiimotes_hid_service();
        }

        let hid_api = if let Some(hid_api) = &mut self.hid_api {
            hid_api.refresh_devices()?;
            hid_api
        } else {
            self.hid_api = Some(HidApi::new()?);
            self.hid_api.as_mut().unwrap_or_else(|| unreachable!())
        };

        let detected_devices = hid_api
            .device_list()
            .filter(|&device_info| WiimoteDevice::get_wiimote_device_type(device_info).is_ok())
            .filter_map(|device_info| {
                device_info
                    .serial_number()
                    .map(ToString::to_string)
                    .map(WiimoteSerialNumber)
                    .map(|serial| (device_info, serial))
            })
            .collect::<Vec<_>>();

        // Disconnected devices
        for removed_serial in self
            .devices
            .keys()
            .cloned()
            .collect::<HashSet<_>>()
            .difference(&detected_devices.iter().map(|(_, s)| s.clone()).collect())
        {
            if let Some(device) = self.devices.get(removed_serial) {
                let mut device = match device.lock() {
                    Ok(d) => d,
                    Err(e) => e.into_inner(),
                };
                if device.is_connected() {
                    device.disconnected();
                }
            }
        }

        let mut new_devices = Vec::new();
        let mut reconnected_devices = Vec::new();
        for (device_info, serial) in detected_devices {
            if self.devices.contains_key(&serial) {
                let previous_device_arc =
                    self.devices.get(&serial).unwrap_or_else(|| unreachable!());

                // Reconnected device
                let reconnected = {
                    let mut previous_device = match previous_device_arc.lock() {
                        Ok(d) => d,
                        Err(d) => d.into_inner(),
                    };
                    if previous_device.is_connected() {
                        false
                    } else {
                        previous_device.reconnect(device_info, hid_api).is_ok()
                    }
                };
                if reconnected {
                    reconnected_devices.push(Arc::clone(previous_device_arc));
                }
            } else {
                // New device
                if let Ok(device) = WiimoteDevice::new(device_info, hid_api) {
                    let device = Arc::new(Mutex::new(device));
                    let cloned_device = Arc::clone(&device);
                    new_devices.push(cloned_device);
                    self.devices.insert(serial.clone(), device);
                }
            }
        }

        Ok(ScanResult {
            new_devices,
            reconnected_devices,
        })
    }

    /// Collection of managed Wii remotes, may contains disconnected ones.
    #[must_use]
    pub fn managed_devices(&self) -> Vec<MutexWiimoteDevice> {
        self.devices.values().map(Arc::clone).collect()
    }

    /// Channel to receive newly connected Wii remotes.
    #[must_use]
    pub fn new_devices_receiver(&self) -> crossbeam_channel::Receiver<MutexWiimoteDevice> {
        self.new_devices_receiver.clone()
    }

    /// Channel to receive reconnected Wii remotes.
    /// Will return the same device as `new_devices_receiver` if the device was disconnected and reconnected.
    #[must_use]
    pub fn reconnected_devices_receiver(&self) -> crossbeam_channel::Receiver<MutexWiimoteDevice> {
        self.reconnected_devices_receiver.clone()
    }
}

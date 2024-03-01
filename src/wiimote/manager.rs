use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once};
use std::thread::JoinHandle;
use std::time::Duration;

use super::device::{NativeWiimote, WiimoteDevice};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct WiimoteSerialNumber(pub String);

extern "C" {
    fn wiimotes_scan() -> u32;
    fn wiimotes_get_next() -> *mut NativeWiimote;
    fn wiimotes_scan_cleanup();
}

type MutexWiimoteDevice = Arc<Mutex<WiimoteDevice>>;

struct ScanResult {
    new_devices: Vec<MutexWiimoteDevice>,
    reconnected_devices: Vec<MutexWiimoteDevice>,
}

/// Periodically checks for connections / disconnections of Wii remotes.
pub struct WiimoteManager {
    devices: HashMap<WiimoteSerialNumber, MutexWiimoteDevice>,
    scan_thread: Option<JoinHandle<()>>,
    scan_interval: Duration,
    new_devices_receiver: crossbeam_channel::Receiver<MutexWiimoteDevice>,
    reconnected_devices_receiver: crossbeam_channel::Receiver<MutexWiimoteDevice>,
}

impl WiimoteManager {
    /// Get the Wii remote manager instance.
    pub fn get_instance() -> Arc<Mutex<Self>> {
        static mut SINGLETON: Option<Arc<Mutex<WiimoteManager>>> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                let instance = Self::new();

                SINGLETON = Some(instance);
            });

            SINGLETON.clone().unwrap_or_else(|| unreachable!())
        }
    }

    fn new() -> Arc<Mutex<Self>> {
        Self::with_interval(Duration::from_millis(100))
    }

    /// Cleanup the Wii remote manager instance and disconnect all Wii remotes.
    pub fn cleanup() {
        unsafe {
            wiimotes_scan_cleanup();
        }
    }

    fn with_interval(interval: Duration) -> Arc<Mutex<Self>> {
        let (new_sender, new_devices_receiver) = crossbeam_channel::unbounded();
        let (reconnected_sender, reconnected_devices_receiver) = crossbeam_channel::unbounded();

        let manager = {
            let mut manager = Self {
                devices: HashMap::new(),
                scan_thread: None,
                scan_interval: interval,
                new_devices_receiver,
                reconnected_devices_receiver,
            };

            // Immediately scan for devices
            let scan_result = manager.scan();
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

                        let ScanResult {
                            new_devices,
                            reconnected_devices,
                        } = manager.scan();
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

                        manager.scan_interval
                    };

                    std::thread::sleep(interval);
                }
            })
        };

        if let Ok(mut manager) = manager.lock() {
            manager.scan_thread = Some(scan_thread);
        }

        manager
    }

    pub fn set_scan_interval(&mut self, interval: Duration) {
        self.scan_interval = interval;
    }

    /// Scan the Wii remotes connected to your computer.
    fn scan(&mut self) -> ScanResult {
        let wiimote_count = unsafe { wiimotes_scan() };

        let mut new_devices = Vec::new();
        let mut reconnected_devices = Vec::new();

        // TODO: iterate disconnected devices and mark them as disconnected

        for _i in 0..wiimote_count {
            let native_wiimote = unsafe { wiimotes_get_next() };
            if native_wiimote.is_null() {
                break;
            }

            let identifier = WiimoteDevice::get_identifier(native_wiimote);
            let serial_number = WiimoteSerialNumber(identifier);
            if let Some(existing_device) = self.devices.get(&serial_number) {
                let result = existing_device.lock().unwrap().reconnect(native_wiimote);
                match result {
                    Ok(()) => reconnected_devices.push(Arc::clone(existing_device)),
                    Err(error) => eprintln!("Failed to reconnect wiimote: {error:?}"),
                }
            } else {
                match unsafe { WiimoteDevice::new(native_wiimote) } {
                    Ok(device) => {
                        let new_device = Arc::new(Mutex::new(device));
                        new_devices.push(Arc::clone(&new_device));
                        self.devices.insert(serial_number, new_device);
                    }
                    Err(error) => eprintln!("Failed to connect to wiimote: {error:?}"),
                }
            }
        }

        ScanResult {
            new_devices,
            reconnected_devices,
        }
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

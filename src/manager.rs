use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use once_cell::sync::Lazy;

use crate::device::WiimoteDevice;
use crate::native::{wiimotes_scan, wiimotes_scan_cleanup, NativeWiimote};

type MutexWiimoteDevice = Arc<Mutex<WiimoteDevice>>;

/// Manages connections to Wii remotes.
/// Periodically checks for new connections of Wii remotes.
pub struct WiimoteManager {
    seen_devices: HashMap<String, MutexWiimoteDevice>,
    scan_interval: Duration,
    new_devices_receiver: crossbeam_channel::Receiver<MutexWiimoteDevice>,
}

impl WiimoteManager {
    /// Get the Wii remote manager instance.
    pub fn get_instance() -> Arc<Mutex<Self>> {
        static mut SINGLETON: Lazy<Arc<Mutex<WiimoteManager>>> =
            Lazy::new(|| WiimoteManager::new_with_interval(Duration::from_millis(500)));
        unsafe { SINGLETON.clone() }
    }

    /// Cleanup the Wii remote manager instance and disconnect all Wii remotes.
    pub fn cleanup() {
        {
            let manager = Self::get_instance();
            let mut manager = match manager.lock() {
                Ok(m) => m,
                Err(m) => m.into_inner(),
            };
            manager.seen_devices.clear();
        }
        wiimotes_scan_cleanup();
    }

    /// Set the interval at which the manager scans for Wii remotes.
    pub fn set_scan_interval(&mut self, scan_interval: Duration) {
        self.scan_interval = scan_interval;
    }

    /// Collection of Wii remotes that are connected or have been connected previously.
    #[must_use]
    pub fn seen_devices(&self) -> Vec<MutexWiimoteDevice> {
        self.seen_devices.values().map(Arc::clone).collect()
    }

    /// Receiver of newly connected Wii remotes.
    #[must_use]
    pub fn new_devices_receiver(&self) -> crossbeam_channel::Receiver<MutexWiimoteDevice> {
        self.new_devices_receiver.clone()
    }

    fn new_with_interval(scan_interval: Duration) -> Arc<Mutex<Self>> {
        let (new_devices_sender, new_devices_receiver) = crossbeam_channel::unbounded();

        let manager = Arc::new(Mutex::new(Self {
            seen_devices: HashMap::new(),
            scan_interval,
            new_devices_receiver,
        }));

        let weak_manager = Arc::downgrade(&manager);
        std::thread::Builder::new()
            .name("wii-remote-scan".to_string())
            .spawn(move || {
                while let Some(manager) = weak_manager.upgrade() {
                    let interval = {
                        let mut manager = match manager.lock() {
                            Ok(m) => m,
                            Err(m) => m.into_inner(),
                        };

                        let new_devices = manager.scan();
                        let send_result = new_devices
                            .into_iter()
                            .try_for_each(|device| new_devices_sender.send(device));
                        if send_result.is_err() {
                            // Channel is disconnected, end scan thread
                            return;
                        }

                        manager.scan_interval
                    };

                    std::thread::sleep(interval);
                }
            })
            .expect("Failed to spawn Wii remote scan thread");

        manager
    }

    /// Scan for connected Wii remotes.
    fn scan(&mut self) -> Vec<MutexWiimoteDevice> {
        let mut native_devices = Vec::new();
        wiimotes_scan(&mut native_devices);

        let mut new_devices = Vec::new();

        for native_wiimote in native_devices {
            let identifier = native_wiimote.identifier();
            if let Some(existing_device) = self.seen_devices.get(&identifier) {
                let result = existing_device.lock().unwrap().reconnect(native_wiimote);
                if let Err(error) = result {
                    eprintln!("Failed to reconnect wiimote: {error:?}");
                }
            } else {
                match WiimoteDevice::new(native_wiimote) {
                    Ok(device) => {
                        let new_device = Arc::new(Mutex::new(device));
                        new_devices.push(Arc::clone(&new_device));
                        self.seen_devices.insert(identifier, new_device);
                    }
                    Err(error) => eprintln!("Failed to connect to wiimote: {error:?}"),
                }
            }
        }

        new_devices
    }
}

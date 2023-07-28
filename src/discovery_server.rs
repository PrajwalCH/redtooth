mod local;

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::protocol::DeviceAddress;
use crate::protocol::DeviceID;

type DeviceMap = HashMap<DeviceID, DeviceAddress>;
type ThreadHandle = JoinHandle<io::Result<()>>;

#[allow(dead_code)]
pub struct DiscoveryServer {
    discovered_devices: Arc<Mutex<DeviceMap>>,
    local_server_handle: Option<ThreadHandle>,
}

impl DiscoveryServer {
    pub fn new() -> DiscoveryServer {
        Self {
            discovered_devices: Arc::new(Mutex::new(DeviceMap::new())),
            local_server_handle: None,
        }
    }

    /// Starts a server for discovering devices on either local or global or both networks.
    pub fn start(&mut self) -> io::Result<()> {
        let thread_handle = local::spawn(Arc::clone(&self.discovered_devices))?;
        self.local_server_handle = Some(thread_handle);
        Ok(())
    }

    /// Announces the device to other instances of the server.
    pub fn announce_device(&self, id: DeviceID, address: DeviceAddress) -> io::Result<()> {
        local::announce_device(id, address)
    }

    /// Returns the identifiers of all the discovered devices.
    pub fn get_discovered_device_ids(&self) -> Option<Vec<DeviceID>> {
        self.discovered_devices.lock().ok().and_then(|device_map| {
            (!device_map.is_empty()).then(|| device_map.keys().copied().collect())
        })
    }

    /// Returns a list of addresses for all the discovered devices.
    pub fn get_discovered_device_addrs(&self) -> Option<Vec<DeviceAddress>> {
        self.discovered_devices.lock().ok().and_then(|device_map| {
            (!device_map.is_empty()).then(|| device_map.values().copied().collect())
        })
    }

    /// Returns the address of a specific device that matches the given identifier.
    pub fn find_device_addr_by_id(&self, id: DeviceID) -> Option<DeviceAddress> {
        self.discovered_devices
            .lock()
            .ok()
            .and_then(|device_map| device_map.get(&id).copied())
    }
}

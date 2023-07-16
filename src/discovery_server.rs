mod local;

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};

use crate::device::{DeviceAddress, DeviceID};

type DeviceMap = HashMap<DeviceID, DeviceAddress>;

pub struct DiscoveryServer {
    device_id: DeviceID,
    device_address: DeviceAddress,
    discovered_devices: Arc<Mutex<DeviceMap>>,
}

impl DiscoveryServer {
    pub fn new(device_id: DeviceID, device_address: DeviceAddress) -> DiscoveryServer {
        Self {
            device_id,
            device_address,
            discovered_devices: Arc::new(Mutex::new(DeviceMap::new())),
        }
    }

    /// Starts a server for discovering devices on either local or global or both network.
    pub fn start(&self) -> io::Result<()> {
        local::start(Arc::clone(&self.discovered_devices))
    }

    /// Announces the device to other instances of the server.
    pub fn announce_device(&self) -> io::Result<()> {
        local::announce_device(self.device_id, self.device_address)
    }
}

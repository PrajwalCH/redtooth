mod local;

use std::io;

use crate::app::EventEmitter;
use crate::device::{DeviceAddress, DeviceID};

/// Starts a server for discovering devices on either local or global or both network.
pub fn start(event_emitter: EventEmitter) -> io::Result<()> {
    local::start(event_emitter)
}

/// Announces the device to other instances of the server.
pub fn announce_device(id: DeviceID, address: DeviceAddress) -> io::Result<()> {
    local::announce_device(id, address)
}

mod device;
mod discovery_server;
mod interface;

use std::io::{self, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use crate::device::Device;
use crate::discovery_server::DiscoveryServer;

fn main() -> io::Result<()> {
    let current_device = Device::new();
    current_device.start_data_receiver()?;

    let mut discovery_server = DiscoveryServer::new();
    discovery_server.start_local_discovery()?;
    discovery_server.announce_device(current_device.id, current_device.address)?;

    while discovery_server.add_discovered_device() {
        // Send ping message to all the devices.
        for peer_address in discovery_server.discovered_devices.values() {
            println!("[Main]: Sending `ping` to {peer_address}");

            let mut peer_stream = TcpStream::connect(peer_address)?;
            peer_stream.write_all("ping".as_bytes())?;
        }
        thread::sleep(Duration::from_secs(2));
    }
    Ok(())
}

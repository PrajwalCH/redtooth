mod device;
mod discovery_server;
mod interface;

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use crate::device::Device;
use crate::discovery_server::DiscoveryServer;

fn main() -> io::Result<()> {
    let current_device = Device::new();
    let mut discovery_server = DiscoveryServer::new();
    discovery_server.start_local_discovery()?;
    discovery_server.announce_device(current_device.id, current_device.address)?;

    let builder = thread::Builder::new().name(String::from("data receiver"));
    let listener = TcpListener::bind(current_device.address)?;
    println!("[Main]: Receiving data on: {}", listener.local_addr()?);

    builder.spawn(move || {
        for peer_stream in listener.incoming() {
            let Ok(mut peer_stream) = peer_stream else {
                continue;
            };

            // NOTE: For now the buffer is only used for holding `ping` message.
            let mut data_buffer = [0; 6];
            peer_stream.read_exact(&mut data_buffer).ok();

            let data = std::str::from_utf8(&data_buffer).unwrap();
            let peer_address = peer_stream.peer_addr().unwrap();

            if !data_buffer.is_empty() {
                println!("[Main]: Received `{data}` from {peer_address}");
            }
        }
    })?;

    loop {
        if let Ok((id, address)) = discovery_server.try_get_discovered_device() {
            discovery_server.add_new_device(id, address);
        }

        // Send ping message to all the devices.
        for peer_address in discovery_server.discovered_devices.values() {
            println!("[Main]: Sending `ping` to {peer_address}");

            let mut peer_stream = TcpStream::connect(peer_address)?;
            peer_stream.write_all("ping".as_bytes())?;
        }
        thread::sleep(Duration::from_secs(2));
    }
}

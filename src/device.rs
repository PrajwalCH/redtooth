use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::thread;

use crate::interface;

const TCP_PORT: u16 = 25802;

pub type Id = u64;
pub type Address = SocketAddr;

pub struct Device {
    pub id: Id,
    pub address: Address,
}

impl Device {
    /// Creates a new instance representing current device.
    pub fn new() -> Self {
        let address = Address::new(
            IpAddr::V4(interface::local_ipv4_address().unwrap_or(Ipv4Addr::UNSPECIFIED)),
            TCP_PORT,
        );
        let id = {
            let mut hasher = DefaultHasher::new();
            address.hash(&mut hasher);
            hasher.finish()
        };

        Self { id, address }
    }

    /// Starts a TCP server for receiving data.
    pub fn start_data_receiver(&self) -> io::Result<()> {
        let builder = thread::Builder::new().name(String::from("data receiver"));
        let listener = TcpListener::bind(self.address)?;
        println!("[Me]: Receiving data on: {}", listener.local_addr()?);

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
        Ok(())
    }
}

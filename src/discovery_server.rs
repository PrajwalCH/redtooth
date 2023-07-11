use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use crate::device::Address as DeviceAddress;
use crate::device::Id as DeviceId;

pub type DiscoveredDevice = (DeviceId, DeviceAddress);

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;

pub struct DiscoveryServer {
    channel: Channel<DiscoveredDevice>,
    pub discovered_devices: HashMap<DeviceId, DeviceAddress>,
}

impl DiscoveryServer {
    pub fn new() -> Self {
        Self {
            discovered_devices: HashMap::new(),
            channel: Channel::new(),
        }
    }

    /// Adds a new device to the list of discovered devices.
    pub fn add_new_device(&mut self, id: DeviceId, address: DeviceAddress) {
        self.discovered_devices.insert(id, address);
    }

    /// Attempts to return a discovered device on the local network.
    pub fn try_get_discovered_device(&self) -> Result<DiscoveredDevice, TryRecvError> {
        self.channel.receiver.try_recv()
    }

    /// Announces the device to other instances of the server.
    pub fn announce_device(&self, id: DeviceId, address: DeviceAddress) -> io::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        // Don't announce to current instance of the server.
        socket.set_multicast_loop_v4(false)?;

        let packet = format!("{};{}", id, address);
        socket.send_to(packet.as_bytes(), (MULTICAST_ADDRESS, MULTICAST_PORT))?;
        Ok(())
    }

    /// Starts a server for discovering devices on the local network.
    pub fn start_local_discovery(&self) -> io::Result<()> {
        let sender = self.channel.sender.clone();
        let builder = thread::Builder::new().name(String::from("local discovery"));
        builder.spawn(move || DiscoveryServer::discover_local_devices(sender))?;
        Ok(())
    }

    /// Starts listening for an **announcement** packet on the local network and sends discovered
    /// device through the channel's sender.
    fn discover_local_devices(sender: Sender<DiscoveredDevice>) -> io::Result<()> {
        let socket = UdpSocket::bind(("0.0.0.0", MULTICAST_PORT))?;
        // socket.set_read_timeout(Some(Duration::from_millis(20)))?;
        socket.join_multicast_v4(&MULTICAST_ADDRESS, &Ipv4Addr::UNSPECIFIED)?;

        println!(
            "[Group]: Listening for new announcement on: {}",
            socket.local_addr()?
        );

        loop {
            let mut packet = [0; 4096];
            let Ok((packet_len, announcement_address)) = socket.recv_from(&mut packet) else {
                continue;
            };
            let Some((id, mut address)) = Self::parse_packet(&packet[..packet_len]) else {
                eprintln!("[Group]: Received invalid formatted packet from {announcement_address}");
                continue;
            };

            // If the address present in a packet is unspecified (0.0.0.0), use the address from
            // which the device announces itself.
            if address.ip().is_unspecified() {
                address.set_ip(announcement_address.ip());
            }
            println!("[Group]: New announcement: [{id}]:[{address}]",);

            if let Err(error) = sender.send((id, address)) {
                eprintln!("[Group]: Couldn't send device id and address to channel: {error}");
                continue;
            }
        }
    }

    /// Parses the packet and returns a [`DiscoveredDevice`] containing the id and address.
    ///
    /// ## Panics
    ///
    /// If the packet is not a valid UTF-8.
    fn parse_packet(packet: &[u8]) -> Option<DiscoveredDevice> {
        let packet = String::from_utf8(packet.to_vec()).unwrap();
        let mut content_iter = packet.split(';');

        let id = content_iter.next()?.parse::<DeviceId>().ok()?;
        let address = content_iter.next()?.parse::<DeviceAddress>().ok()?;
        Some((id, address))
    }
}

struct Channel<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<T>();
        Self { sender, receiver }
    }
}

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use crate::interface;

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;
const TCP_PORT: u16 = 25802;

type DeviceId = u64;
type DeviceAddress = SocketAddr;

pub struct Group {
    pub current_device: Device,
    pub joined_devices: HashMap<DeviceId, DeviceAddress>,
    channel: Channel<Device>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            current_device: Device::current(),
            joined_devices: HashMap::new(),
            channel: Channel::new(),
        }
    }

    /// Adds a new device to the list of joined devices.
    pub fn add_new_device(&mut self, device: Device) {
        self.joined_devices.insert(device.id, device.address);
    }

    /// Attempts to return a discovered device on the local network.
    pub fn try_get_discovered_device(&self) -> Result<Device, TryRecvError> {
        self.channel.receiver.try_recv()
    }

    /// Announces the current device to other instances of the group server.
    pub fn announce_current_device(&self) -> io::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        // Don't announce self to own group server.
        socket.set_multicast_loop_v4(false)?;

        let packet = format!("{};{}", self.current_device.id, self.current_device.address);
        socket.send_to(packet.as_bytes(), (MULTICAST_ADDRESS, MULTICAST_PORT))?;
        Ok(())
    }

    /// Starts a server for discovering devices on the local network.
    pub fn start_local_discovery(&self) -> io::Result<()> {
        let sender = self.channel.sender.clone();
        let builder = thread::Builder::new().name(String::from("local discovery"));
        builder.spawn(move || Group::discover_local_devices(sender))?;
        Ok(())
    }

    /// Starts listening for an **announcement** packet on the local network and sends discovered
    /// device through the channel's sender.
    fn discover_local_devices(sender: Sender<Device>) -> io::Result<()> {
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
            let Some(mut device) = Self::parse_packet(&packet[..packet_len]) else {
                eprintln!("[Group]: Received invalid formatted packet from {announcement_address}");
                continue;
            };

            // If the address present in a packet is unspecified (0.0.0.0), use the address from
            // which the device announces itself.
            if device.address.ip().is_unspecified() {
                device.address.set_ip(announcement_address.ip());
            }
            println!(
                "[Group]: New announcement: [{}]:[{}]",
                device.id, device.address
            );

            if let Err(error) = sender.send(device) {
                eprintln!("[Group]: Couldn't send device id and address to channel: {error}");
                continue;
            }
        }
    }

    /// Parses the packet and returns a [`Device`] containing the id and address.
    ///
    /// ## Panics
    ///
    /// If the packet is not a valid UTF-8.
    fn parse_packet(packet: &[u8]) -> Option<Device> {
        let packet = String::from_utf8(packet.to_vec()).unwrap();
        let mut content_iter = packet.split(';');

        let id = DeviceId::from_str(content_iter.next()?).ok()?;
        let address = DeviceAddress::from_str(content_iter.next()?).ok()?;
        Some(Device::new(id, address))
    }
}

pub struct Device {
    pub id: DeviceId,
    pub address: DeviceAddress,
}

impl Device {
    pub fn new(id: DeviceId, address: DeviceAddress) -> Self {
        Self { id, address }
    }

    /// Creates a new instance representing current device.
    pub fn current() -> Self {
        let address = DeviceAddress::new(
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

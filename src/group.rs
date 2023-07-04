use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;
const ANY_INTERFACE_ADDRESS: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const TCP_PORT: u16 = 25802;

type DeviceId = u64;
type DeviceAddress = SocketAddr;

pub struct Group {
    /// Current device address.
    pub device_address: DeviceAddress,
    pub joined_devices: HashMap<DeviceId, DeviceAddress>,
    channel: Channel<(DeviceId, DeviceAddress)>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            device_address: DeviceAddress::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), TCP_PORT),
            joined_devices: HashMap::new(),
            channel: Channel::new(),
        }
    }

    /// Announces the current device to other instances of the group server.
    pub fn announce(&self) -> io::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        // Don't announce self to own group server.
        socket.set_multicast_loop_v4(false)?;
        socket.send_to(
            "announcement".as_bytes(),
            (MULTICAST_ADDRESS, MULTICAST_PORT),
        )?;
        Ok(())
    }

    /// Adds a new device to the list of joined devices.
    pub fn add_new_device(&mut self, device_id: DeviceId, device_address: DeviceAddress) {
        self.joined_devices.insert(device_id, device_address);
    }

    /// Attempts to return a discovered device on the local network.
    pub fn try_get_discovered_device(&self) -> Result<(DeviceId, DeviceAddress), TryRecvError> {
        self.channel.receiver.try_recv()
    }

    /// Starts a server for discovering devices on the local network.
    pub fn start_local_discovery(&self) -> io::Result<()> {
        let sender = self.channel.sender.clone();
        let builder = thread::Builder::new().name(String::from("local discovery"));
        builder.spawn(move || Group::discover_devices(sender))?;
        Ok(())
    }

    /// Starts listening for an **announcement** packet and sends the id and address of a discovered
    /// device through the channel's sender.
    fn discover_devices(sender: Sender<(DeviceId, DeviceAddress)>) -> io::Result<()> {
        let socket = UdpSocket::bind(("0.0.0.0", MULTICAST_PORT))?;
        // socket.set_read_timeout(Some(Duration::from_millis(20)))?;
        socket.join_multicast_v4(&MULTICAST_ADDRESS, &ANY_INTERFACE_ADDRESS)?;

        println!(
            "[Group]: Listening for new announcement on: {}",
            socket.local_addr()?
        );

        loop {
            let mut inbox = [0; 12];
            let Ok((_, mut device_address)) = socket.recv_from(&mut inbox) else {
                continue;
            };

            // All the devices receive data on a fixed `TCP_PORT` but the address currently we are
            // receiving may have different port because they announce themselves using the port
            // chosen by OS or a specifically `UNSPECIFIED` port.
            device_address.set_port(TCP_PORT);

            if inbox != "announcement".as_bytes() {
                continue;
            }
            let device_id = Self::generate_device_id(device_address);
            if let Err(error) = sender.send((device_id, device_address)) {
                eprintln!("[Group]: Couldn't send device id and address to channel: {error}");
                continue;
            }
            println!("[Group]: New announcement: [{device_id}]:[{device_address}]");
        }
    }

    fn generate_device_id(device_address: impl Hash) -> DeviceId {
        let mut hasher = DefaultHasher::new();
        device_address.hash(&mut hasher);
        hasher.finish()
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

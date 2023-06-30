use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::mpsc::{self, Sender};
use std::thread;

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;
const ANY_INTERFACE_ADDRESS: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

type DeviceId = u64;
type DeviceAddress = SocketAddr;

struct Group {
    joined_devices: HashMap<DeviceId, DeviceAddress>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            joined_devices: HashMap::new(),
        }
    }

    pub fn add_new_device(&mut self, device_id: DeviceId, device_address: DeviceAddress) {
        self.joined_devices.insert(device_id, device_address);
    }

    pub fn announce() -> io::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        // Don't announce self to own group server.
        socket.set_multicast_loop_v4(false)?;
        socket.send_to(
            "announcement".as_bytes(),
            (MULTICAST_ADDRESS, MULTICAST_PORT),
        )?;
        Ok(())
    }

    pub fn listen_new_announcement(sender: &Sender<(DeviceId, DeviceAddress)>) -> io::Result<()> {
        let socket = UdpSocket::bind(("0.0.0.0", MULTICAST_PORT))?;
        // socket.set_read_timeout(Some(Duration::from_millis(20)))?;
        socket.join_multicast_v4(&MULTICAST_ADDRESS, &ANY_INTERFACE_ADDRESS)?;

        println!(
            "[Group]: Listening for new announcement on: {}",
            socket.local_addr()?
        );

        loop {
            let mut inbox = [0; 12];
            let Ok((_, device_address)) = socket.recv_from(&mut inbox) else {
                continue;
            };

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

fn main() -> io::Result<()> {
    let (sender, receiver) = mpsc::channel::<(DeviceId, DeviceAddress)>();
    let builder = thread::Builder::new().name(String::from("announcer"));
    builder.spawn(move || Group::listen_new_announcement(&sender))?;
    // Announce self to other group server instances.
    Group::announce()?;

    let mut group = Group::new();

    loop {
        if let Ok(member) = receiver.recv() {
            group.add_new_device(member.0, member.1);
        }
    }
}

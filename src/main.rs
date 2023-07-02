use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;
const ANY_INTERFACE_ADDRESS: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const TCP_PORT: u16 = 25802;

type DeviceId = u64;
type DeviceAddress = SocketAddr;

struct Group {
    /// Current device address.
    device_address: DeviceAddress,
    joined_devices: HashMap<DeviceId, DeviceAddress>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            device_address: DeviceAddress::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), TCP_PORT),
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

fn main() -> io::Result<()> {
    let (sender, receiver) = mpsc::channel::<(DeviceId, DeviceAddress)>();
    let builder = thread::Builder::new().name(String::from("announcer"));
    builder.spawn(move || Group::listen_new_announcement(&sender))?;

    let mut group = Group::new();
    // Announce self to other group server instances.
    Group::announce()?;

    let listener = TcpListener::bind(group.device_address)?;
    println!("[Main]: Receiving data on: {}", listener.local_addr()?);

    let builder = thread::Builder::new().name(String::from("data receiver"));

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
        if let Ok((device_id, device_address)) = receiver.try_recv() {
            group.add_new_device(device_id, device_address);
        }

        // Send ping message to all the devices.
        for peer_address in group.joined_devices.values() {
            println!("[Main]: Sending `ping` to {peer_address}");

            let mut peer_stream = TcpStream::connect(peer_address)?;
            peer_stream.write_all("ping".as_bytes())?;
        }
        thread::sleep(Duration::from_secs(2));
    }
}

use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::thread;

use crate::app::DeviceAddress;
use crate::app::DeviceID;
use crate::app::Event;
use crate::app::EventEmitter;

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;

/// Announces the device to other instances of the server.
pub fn announce_device(id: DeviceID, address: DeviceAddress) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    // Don't announce to current instance of the server.
    socket.set_multicast_loop_v4(false)?;

    let packet = format!("{};{}", id, address);
    socket.send_to(packet.as_bytes(), (MULTICAST_ADDRESS, MULTICAST_PORT))?;
    Ok(())
}

/// Starts a server for discovering devices on the local network.
pub fn start_local_discovery(event_emitter: EventEmitter) -> io::Result<()> {
    let builder = thread::Builder::new().name(String::from("local discovery"));
    builder.spawn(move || discover_local_devices(event_emitter))?;
    Ok(())
}

/// Starts listening for an **announcement** packet on the local network and emits the event
/// when new device is discovered.
fn discover_local_devices(event_emitter: EventEmitter) -> io::Result<()> {
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
        let Some((id, mut address)) = parse_packet(&packet[..packet_len]) else {
            eprintln!("[Group]: Received invalid formatted packet from {announcement_address}");
            continue;
        };

        // If the address present in a packet is unspecified (0.0.0.0), use the address from
        // which the device announces itself.
        if address.ip().is_unspecified() {
            address.set_ip(announcement_address.ip());
        }
        println!("[Group]: New announcement: [{id}]:[{address}]",);

        if let Err(error) = event_emitter.emit(Event::DiscoveredNewDevice((id, address))) {
            eprintln!("[Group]: Couldn't send device id and address to channel: {error}");
            continue;
        }
    }
}

/// Parses the packet and returns the id and address.
///
/// ## Panics
///
/// If the packet is not a valid UTF-8.
fn parse_packet(packet: &[u8]) -> Option<(DeviceID, DeviceAddress)> {
    let packet = String::from_utf8(packet.to_vec()).unwrap();
    let mut content_iter = packet.split(';');

    let id = content_iter.next()?.parse::<DeviceID>().ok()?;
    let address = content_iter.next()?.parse::<DeviceAddress>().ok()?;
    Some((id, address))
}

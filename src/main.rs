mod group;

use group::{DeviceAddress, DeviceId, Group};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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

use std::io::{self, Read};
use std::net::TcpListener;

use crate::app::{Event, EventEmitter};
use crate::protocol::{DeviceAddress, FilePacket};
use crate::{elogln, logln};

pub fn start_file_receiving(addr: DeviceAddress, event_emitter: EventEmitter) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    logln!("Receiving data on {addr}");

    for mut stream in listener.incoming().flatten() {
        let mut data: Vec<u8> = Vec::new();

        match stream.read_to_end(&mut data) {
            Ok(data_len) => logln!("Received data of {data_len} bytes"),
            Err(e) => elogln!("Couldn't read data from the stream: {e}"),
        };

        match FilePacket::from_bytes(&data) {
            Ok(file_packet) => event_emitter.emit(Event::FileReceived(file_packet)),
            Err(e) => elogln!("Received data isn't a valid file packet; {e}"),
        };
    }
    Ok(())
}

use std::fs;
use std::io::{self, Read};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use crate::protocol::{DeviceAddress, FilePacket};
use crate::{elogln, logln};

/// Starts receiving files on the `addr` and upon successful reception saves them
/// to the given location.
pub fn receive_files(addr: DeviceAddress, save_location: PathBuf) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    logln!("Receiving data on {addr}");

    for mut stream in listener.incoming().flatten() {
        let mut data: Vec<u8> = Vec::new();

        match stream.read_to_end(&mut data) {
            Ok(data_len) => logln!("Received data of {data_len} bytes"),
            Err(e) => elogln!("Couldn't read data from the stream: {e}"),
        };

        let file_packet = match FilePacket::from_bytes(&data) {
            Ok(p) => p,
            Err(e) => {
                elogln!("Received data isn't a valid file packet; {e}");
                continue;
            }
        };

        if let Err(e) = write_file(file_packet, &save_location) {
            let path = save_location.display();
            elogln!("Failed to create file in `{path}`: {e}");
        }
    }
    Ok(())
}

/// Creates a file based on the provided file packet.
///
/// This function will create a file if it does not exist,
/// and will entirely replace its contents with a new one if it does.
fn write_file(packet: FilePacket, save_location: &Path) -> io::Result<()> {
    let file_path = save_location.join(packet.header.file_name);
    fs::write(file_path, packet.contents)
}

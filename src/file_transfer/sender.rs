use std::io::{self, Write};
use std::net::TcpStream;
use std::path::Path;

use super::FilePacket;
use crate::logln;
use crate::protocol::PeerAddr;

pub fn send_file_to(addr: PeerAddr, path: impl AsRef<Path>) -> io::Result<()> {
    send_file_to_all(&[addr], path)
}

pub fn send_file_to_all(addrs: &[PeerAddr], path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    assert!(path.is_file());

    let packet = FilePacket::from_path(path)?;
    let data = packet.as_owned_bytes();
    logln!("Sending data of {} bytes", data.len());

    for addr in addrs {
        let mut stream = TcpStream::connect(addr)?;
        stream.write_all(&data)?;
    }
    Ok(())
}

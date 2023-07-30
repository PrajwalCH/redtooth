use std::fs;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;

use crate::logln;
use crate::protocol::{FilePacket, FilePacketHeader, PeerAddr};

pub fn send_file_to<P: AsRef<Path>>(addr: PeerAddr, path: P) -> io::Result<()> {
    send_file_to_all(&[addr], path)
}

pub fn send_file_to_all<P: AsRef<Path>>(addrs: &[PeerAddr], path: P) -> io::Result<()> {
    let path = path.as_ref();
    assert!(path.is_file());

    let file_name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_string();
    let header = FilePacketHeader { file_name };
    let file_contents = fs::read(path)?;
    let data = FilePacket::new(header, &file_contents).as_owned_bytes();
    logln!("Sending data of {} bytes", data.len());

    for addr in addrs {
        let mut stream = TcpStream::connect(addr)?;
        stream.write_all(&data)?;
    }
    Ok(())
}

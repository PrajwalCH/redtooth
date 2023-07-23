use std::fs;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;

use crate::app::DataHeader;
use crate::protocol::DeviceAddress;

pub fn send_file_to<P: AsRef<Path>>(addr: DeviceAddress, path: P) -> io::Result<()> {
    send_file_to_all(&[addr], path)
}

pub fn send_file_to_all<P: AsRef<Path>>(addrs: &[DeviceAddress], path: P) -> io::Result<()> {
    let path = path.as_ref();
    assert!(path.is_file());

    let file_name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_string();
    let header = DataHeader { file_name };
    let header = header.to_string();
    let file_contents = fs::read(path)?;

    let mut data = Vec::new();
    data.extend_from_slice(header.as_bytes());
    data.extend_from_slice(b"::");
    data.extend_from_slice(&file_contents);

    for addr in addrs {
        let mut stream = TcpStream::connect(addr)?;
        stream.write_all(&data)?;
    }
    Ok(())
}

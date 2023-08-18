pub mod receiver;
pub mod sender;

use std::path::Path;
use std::{fs, io, str};

use crate::protocol::packet::{Packet, PacketParseError};

/// A wrapper around [`Packet`] specialized for constructing a packet to send or receive files
/// along with their associated metadata.
pub struct FilePacket<'data>(Packet<'data>);

impl<'data> FilePacket<'data> {
    /// Creates a new file packet by reading a file from the given path.
    pub fn from_path(path: &Path) -> io::Result<FilePacket<'data>> {
        let file_name = path.file_name().unwrap_or(path.as_os_str());
        let file_contents = fs::read(path)?;

        let mut packet = Packet::new();
        packet.set_header("file_name", file_name.to_string_lossy());
        packet.set_payload(file_contents);
        Ok(FilePacket(packet))
    }

    /// Creates a new file packet by parsing the given bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<FilePacket, PacketParseError> {
        Ok(FilePacket(Packet::from_bytes(bytes)?))
    }

    /// Returns the name of the file, if available; otherwise returns default (`undefined`).
    pub fn get_file_name(&self) -> &str {
        self.0.get_header("file_name").unwrap_or("undefined")
    }

    /// Returns the contents of the file, if available; otherwise returns empty.
    pub fn get_contents(&self) -> &[u8] {
        self.0.get_payload().unwrap_or_default()
    }

    /// Converts the packet into a bytes which can be sent over the network.
    pub fn as_owned_bytes(&self) -> Vec<u8> {
        self.0.as_bytes()
    }
}

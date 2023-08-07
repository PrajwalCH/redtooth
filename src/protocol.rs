use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::str::{self, Utf8Error};
use std::time::Instant;
use std::{fs, io};

use crate::interface;

const TCP_PORT: u16 = 25802;
/// Represents a separator used to distinguish sections, such as headers and payload
/// of the packet.
const PACKET_SECTIONS_SEPARATOR: &[u8; 2] = b"::";

pub type PeerID = u64;
pub type PeerAddr = SocketAddr;

pub fn get_my_id() -> PeerID {
    let mut hasher = DefaultHasher::new();
    Instant::now().hash(&mut hasher);
    hasher.finish()
}

pub fn get_my_addr() -> PeerAddr {
    let ip_addr = IpAddr::V4(interface::local_ipv4_address().unwrap_or(Ipv4Addr::UNSPECIFIED));
    PeerAddr::new(ip_addr, TCP_PORT)
}

/// Represents possible errors that can occur when reconstructing a new [`Packet`]
/// from the bytes.
///
/// This error is returned from the [`Packet::from_bytes`].
#[derive(Debug)]
pub enum PacketParseError {
    MissingSectionsSeparator,
    InvalidUtf8(Utf8Error),
}

impl fmt::Display for PacketParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::PacketParseError::*;

        match self {
            MissingSectionsSeparator => write!(f, "missing sections separator"),
            InvalidUtf8(e) => write!(f, "{e}"),
        }
    }
}

/// Represents a packet used for transferring any data along with the additional information.
///
/// The packet is divided into two sections separated by [`PACKET_SECTIONS_SEPARATOR`]:
///
/// - **Headers** allow the sender and receiver to either pass additional information for the
/// communication or to pass more information about the data to be transmitted.
///
/// - **Payload** holds the actual data to be transmitted.
pub struct Packet<'p> {
    headers: HashMap<String, String>,
    payload: Option<Cow<'p, [u8]>>,
}

impl<'p> Packet<'p> {
    /// Creates a new empty packet.
    pub fn new() -> Packet<'p> {
        Packet {
            headers: HashMap::new(),
            payload: None,
        }
    }

    /// Creates a new packet by preserving its state from the given bytes.
    ///
    /// This function attempts to reconstruct a new [`Packet`] from the provided bytes
    /// with the same state as it was originally created using [`Packet::as_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Packet, PacketParseError> {
        let separator_len = PACKET_SECTIONS_SEPARATOR.len();
        let separator_index = bytes
            .windows(separator_len)
            .position(|bytes| bytes == PACKET_SECTIONS_SEPARATOR)
            .ok_or(PacketParseError::MissingSectionsSeparator)?;

        let headers = str::from_utf8(&bytes[..separator_index])
            .map_err(PacketParseError::InvalidUtf8)?
            .lines()
            .filter_map(|header| header.split_once(':'))
            .map(|(name, value)| (name.to_string(), value.trim().to_string()))
            .collect::<HashMap<String, String>>();
        let payload = bytes
            .get(separator_index + separator_len..)
            .map(Cow::Borrowed);

        Ok(Packet { headers, payload })
    }

    /// Inserts a header into the packet or updates its value if the header already exists.
    pub fn set_header<N, V>(&mut self, name: N, value: V)
    where
        N: ToString,
        V: ToString,
    {
        self.headers.insert(name.to_string(), value.to_string());
    }

    /// Sets the payload to be transmitted.
    pub fn set_payload(&mut self, payload: Vec<u8>) {
        self.payload = Some(Cow::Owned(payload));
    }

    /// Returns a reference to the value corresponding to the header.
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|v| v.as_str())
    }

    /// Returns the payload of the packet, if available.
    pub fn get_payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    /// Converts the packet into a bytes which can be sent over the network.
    ///
    /// These bytes on the receiver side can then be used to reconstruct a new [`Packet`]
    /// using [`Packet::from_bytes`] with the same state as at the time of sending.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut headers = String::new();

        for (name, value) in self.headers.iter() {
            writeln!(headers, "{name}: {value}").unwrap();
        }
        let mut final_bytes = Vec::new();
        final_bytes.extend_from_slice(headers.as_bytes());
        final_bytes.extend_from_slice(PACKET_SECTIONS_SEPARATOR);

        if let Some(payload) = self.get_payload() {
            final_bytes.extend_from_slice(payload);
        }
        final_bytes
    }
}

/// A wrapper around [`Packet`] specialized for constructing a packet to send or receive files
/// along with their associated metadata.
///
/// TODO: This module is not a good place to put this.
pub struct FilePacket<'data>(Packet<'data>);

impl<'data> FilePacket<'data> {
    /// Creates a new empty file packet.
    pub fn from_path(path: &Path) -> io::Result<FilePacket<'data>> {
        let file_name = path.file_name().unwrap_or(path.as_os_str());
        let file_contents = fs::read(path)?;

        let mut packet = Packet::new();
        packet.set_header("file_name", file_name.to_string_lossy());
        packet.set_payload(file_contents);
        Ok(FilePacket(packet))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<FilePacket, PacketParseError> {
        Ok(FilePacket(Packet::from_bytes(bytes)?))
    }

    pub fn get_file_name(&self) -> &str {
        self.0.get_header("file_name").unwrap_or("undefined")
    }

    pub fn get_contents(&self) -> &[u8] {
        self.0.get_payload().unwrap_or_default()
    }

    pub fn as_owned_bytes(&self) -> Vec<u8> {
        self.0.as_bytes()
    }
}

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::Instant;

use crate::interface;

const TCP_PORT: u16 = 25802;
/// Represents a separator used to distinguish sections, such as header and file contents
/// of a file packet.
const PACKET_SECTIONS_SEPARATOR: &[u8; 2] = b"::";

pub type DeviceID = u64;
pub type DeviceAddress = SocketAddr;

pub fn device_id() -> DeviceID {
    let mut hasher = DefaultHasher::new();
    Instant::now().hash(&mut hasher);
    hasher.finish()
}

pub fn device_address() -> DeviceAddress {
    let ip_addr = IpAddr::V4(interface::local_ipv4_address().unwrap_or(Ipv4Addr::UNSPECIFIED));
    DeviceAddress::new(ip_addr, TCP_PORT)
}

/// Represents possible errors that can occur when converting a slice of bytes into a [`FilePacket`].
///
/// This error is returned from the [`FilePacket::from_bytes`].
pub enum FilePacketFromBytesError {
    MissingSectionsSeparator,
    HeaderParseError(FilePacketHeaderParseError),
}

impl fmt::Display for FilePacketFromBytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::FilePacketFromBytesError::*;

        match self {
            MissingSectionsSeparator => {
                write!(f, "missing sections separator")
            }
            HeaderParseError(e) => {
                write!(f, "couldn't parse the header: {e}")
            }
        }
    }
}

/// Represents a packet used for transferring files along with their associated metadata.
///
/// The file packet is divided into two sections separated by [`PACKET_SECTIONS_SEPARATOR`]:
///
/// - **[`FilePacketHeader`]:** The header holds the metadata or information associated with the file.
///   This includes relevant details such as file name, size, checksum, etc.
///
/// - **Contents:** The contents section holds the actual data of the file to be transmitted.
#[derive(Debug)]
pub struct FilePacket {
    /// The header information of the file packet.
    pub header: FilePacketHeader,
    /// The contents of the file.
    pub contents: Vec<u8>,
}

impl FilePacket {
    /// Creates a new file packet with the given header and contents.
    pub fn new(header: FilePacketHeader, contents: Vec<u8>) -> FilePacket {
        Self { header, contents }
    }

    /// Converts a slice of bytes into a file packet.
    pub fn from_bytes(bytes: &[u8]) -> Result<FilePacket, FilePacketFromBytesError> {
        let separator_len = PACKET_SECTIONS_SEPARATOR.len();
        let separator_index = bytes
            .windows(separator_len)
            .position(|bytes| bytes == PACKET_SECTIONS_SEPARATOR)
            .ok_or(FilePacketFromBytesError::MissingSectionsSeparator)?;

        let header = std::str::from_utf8(&bytes[..separator_index]).unwrap_or_default();
        let header = FilePacketHeader::from_str(header)
            .map_err(FilePacketFromBytesError::HeaderParseError)?;
        // Skip all the separator bytes.
        let contents = bytes.get(separator_index + separator_len..);
        // If a valid header and separator are present but the contents are missing,
        // declare it as an empty.
        let contents = contents.unwrap_or_default().to_owned();
        Ok(Self { header, contents })
    }

    /// Converts a file packet into vector of bytes.
    pub fn as_owned_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let header = self.header.to_string();
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(PACKET_SECTIONS_SEPARATOR);
        bytes.extend_from_slice(&self.contents);
        bytes
    }
}

/// Represents possible errors that can occur when parsing a string to a [`FilePacketHeader`].
///
/// This error is returned from the [`FilePacketHeader::from_str`].
pub enum FilePacketHeaderParseError {
    MissingName,
}

impl fmt::Display for FilePacketHeaderParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::FilePacketHeaderParseError::*;

        match self {
            MissingName => write!(f, "missing required field `name`"),
        }
    }
}

/// Represents the header information for a file packet being transmitted.
///
/// It encapsulates essential metadata about the file being sent.
/// This header is pre-pended to the actual file data before transmission,
/// allowing the receiver to correctly handle the incoming data.
#[derive(Debug)]
pub struct FilePacketHeader {
    /// The name of the file, including its extension.
    pub file_name: String,
}

impl FromStr for FilePacketHeader {
    type Err = FilePacketHeaderParseError;

    fn from_str(s: &str) -> Result<FilePacketHeader, FilePacketHeaderParseError> {
        let name = s
            .trim()
            .strip_prefix("file_name: ")
            .ok_or(FilePacketHeaderParseError::MissingName)?
            .to_string();

        Ok(Self { file_name: name })
    }
}

impl fmt::Display for FilePacketHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "file_name: {}", self.file_name)
    }
}

use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Write};
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::{self, Utf8Error};
use std::time::Instant;

use crate::interface;

const TCP_PORT: u16 = 25802;
/// Represents a separator used to distinguish sections, such as header and file contents
/// of a file packet.
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
pub struct FilePacket<'data> {
    /// The header information of the file packet.
    pub header: FilePacketHeader<'data>,
    /// The contents of the file.
    pub contents: &'data [u8],
}

impl<'data> FilePacket<'data> {
    /// Creates a new file packet with the given header and contents.
    pub fn new(header: FilePacketHeader<'data>, contents: &'data [u8]) -> FilePacket<'data> {
        Self { header, contents }
    }

    /// Converts a slice of bytes into a file packet.
    pub fn from_bytes(bytes: &'data [u8]) -> Result<FilePacket, FilePacketFromBytesError> {
        let separator_len = PACKET_SECTIONS_SEPARATOR.len();
        let separator_index = bytes
            .windows(separator_len)
            .position(|bytes| bytes == PACKET_SECTIONS_SEPARATOR)
            .ok_or(FilePacketFromBytesError::MissingSectionsSeparator)?;

        let header = FilePacketHeader::from_bytes(&bytes[..separator_index])
            .map_err(FilePacketFromBytesError::HeaderParseError)?;
        // Skip all the separator bytes.
        let contents = bytes.get(separator_index + separator_len..);
        // If a valid header and separator are present but the contents are missing,
        // declare it as an empty.
        let contents = contents.unwrap_or_default();
        Ok(Self { header, contents })
    }

    /// Converts a file packet into vector of bytes.
    pub fn as_owned_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.as_owned_bytes());
        bytes.extend_from_slice(PACKET_SECTIONS_SEPARATOR);
        bytes.extend_from_slice(self.contents);
        bytes
    }
}

/// Represents possible errors that can occur when parsing a string to a [`FilePacketHeader`].
///
/// This error is returned from the [`FilePacketHeader::from_str`].
pub enum FilePacketHeaderParseError {
    MissingFileName,
    InvalidUtf8(Utf8Error),
}

impl fmt::Display for FilePacketHeaderParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::FilePacketHeaderParseError::*;

        match self {
            MissingFileName => write!(f, "missing required field `file_name`"),
            InvalidUtf8(e) => write!(f, "{e}"),
        }
    }
}

/// Represents the header information for a file packet being transmitted.
///
/// It encapsulates essential metadata about the file being sent.
/// This header is pre-pended to the actual file data before transmission,
/// allowing the receiver to correctly handle the incoming data.
#[derive(Debug)]
pub struct FilePacketHeader<'data> {
    /// The name of the file, including its extension.
    pub file_name: &'data str,
}

impl<'data> FilePacketHeader<'data> {
    /// Creates a new file packet header with the given file name.
    pub fn new(file_name: &'data str) -> FilePacketHeader {
        Self { file_name }
    }

    /// Converts a slice of bytes into a file packet header.
    pub fn from_bytes(b: &'data [u8]) -> Result<FilePacketHeader, FilePacketHeaderParseError> {
        let header = str::from_utf8(b).map_err(FilePacketHeaderParseError::InvalidUtf8)?;
        let file_name = header
            .trim()
            .strip_prefix("file_name: ")
            .ok_or(FilePacketHeaderParseError::MissingFileName)?;

        Ok(Self { file_name })
    }

    /// Converts a file packet header into vector of bytes.
    pub fn as_owned_bytes(&self) -> Vec<u8> {
        let mut data = String::new();
        writeln!(data, "file_name: {}", self.file_name).ok();
        data.as_bytes().to_vec()
    }
}

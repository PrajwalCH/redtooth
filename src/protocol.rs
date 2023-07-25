use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::Instant;

use crate::interface;

const TCP_PORT: u16 = 25802;
/// The data sections separator used to distinguish sections (e.g., header and file contents)
/// within a data stream.
pub const DATA_SECTIONS_SEPARATOR: &[u8; 2] = b"::";

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
    DataHeaderParseError(DataHeaderParseError),
}

impl fmt::Display for FilePacketFromBytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::FilePacketFromBytesError::*;

        match self {
            MissingSectionsSeparator => {
                write!(f, "Sections separator is missing from a file packet")
            }
            DataHeaderParseError(e) => {
                write!(f, "Unable to parse the header of a file packet: {e}")
            }
        }
    }
}

/// Represents a packet used for transferring files along with their associated metadata.
///
/// The file packet is divided into two sections separated by [`DATA_SECTIONS_SEPARATOR`]:
///
/// - **[`DataHeader`]:** The header holds the metadata or information associated with the file.
///   This includes relevant details such as file name, size, checksum, etc.
///
/// - **Contents:** The contents section holds the actual data of the file to be transmitted.
#[derive(Debug)]
pub struct FilePacket {
    /// The header information of the file packet.
    pub header: DataHeader,
    /// The contents of the file.
    pub contents: Vec<u8>,
}

impl FilePacket {
    /// Creates a new file packet with the given header and contents.
    pub fn new(header: DataHeader, contents: Vec<u8>) -> FilePacket {
        Self { header, contents }
    }

    /// Converts a slice of bytes into a file packet.
    pub fn from_bytes(bytes: &[u8]) -> Result<FilePacket, FilePacketFromBytesError> {
        let separator_len = DATA_SECTIONS_SEPARATOR.len();
        let separator_index = bytes
            .windows(separator_len)
            .position(|bytes| bytes == DATA_SECTIONS_SEPARATOR)
            .ok_or(FilePacketFromBytesError::MissingSectionsSeparator)?;

        let header = std::str::from_utf8(&bytes[..separator_index]).unwrap_or_default();
        let header =
            DataHeader::from_str(header).map_err(FilePacketFromBytesError::DataHeaderParseError)?;
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
        bytes.extend_from_slice(DATA_SECTIONS_SEPARATOR);
        bytes.extend_from_slice(&self.contents);
        bytes
    }
}

/// An error returned from [`DataHeader::from_str`].
pub enum DataHeaderParseError {
    MissingName,
}

impl fmt::Display for DataHeaderParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::DataHeaderParseError::*;

        match self {
            MissingName => write!(f, "Missing required `name` field"),
        }
    }
}

/// Represents the header information for data being transmitted.
///
/// It encapsulates essential metadata about the file being sent.
/// This header is pre-pended to the actual file data before transmission,
/// allowing the receiver to correctly handle the incoming data.
#[derive(Debug)]
pub struct DataHeader {
    /// The name of the file, including its extension.
    pub(crate) file_name: String,
}

impl FromStr for DataHeader {
    type Err = DataHeaderParseError;

    fn from_str(s: &str) -> Result<DataHeader, DataHeaderParseError> {
        let name = s
            .trim()
            .strip_prefix("file_name: ")
            .ok_or(DataHeaderParseError::MissingName)?
            .to_string();

        Ok(Self { file_name: name })
    }
}

impl fmt::Display for DataHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "file_name: {}", self.file_name)
    }
}

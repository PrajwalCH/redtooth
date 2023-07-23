use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::Instant;

use crate::interface;

const TCP_PORT: u16 = 25802;
/// The data sections separator used to distinguish sections (eg. header and file contents)
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

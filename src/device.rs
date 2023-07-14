use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Instant;

use crate::interface;

const TCP_PORT: u16 = 25802;

pub type DeviceID = u64;
pub type DeviceAddress = SocketAddr;

pub fn id() -> DeviceID {
    let mut hasher = DefaultHasher::new();
    Instant::now().hash(&mut hasher);
    hasher.finish()
}

pub fn address() -> DeviceAddress {
    let ip_addr = IpAddr::V4(interface::local_ipv4_address().unwrap_or(Ipv4Addr::UNSPECIFIED));
    DeviceAddress::new(ip_addr, TCP_PORT)
}

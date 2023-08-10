pub mod packet;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Instant;

use crate::interface;

const DEFAULT_PEER_IP: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
const DEFAULT_PEER_PORT: u16 = 25802;

pub type PeerID = u64;
pub type PeerAddr = SocketAddr;

pub fn get_my_id() -> PeerID {
    let mut hasher = DefaultHasher::new();
    Instant::now().hash(&mut hasher);
    hasher.finish()
}

pub fn get_my_addr() -> PeerAddr {
    let ip_addr = IpAddr::V4(interface::local_ipv4_address().unwrap_or(DEFAULT_PEER_IP));
    PeerAddr::new(ip_addr, DEFAULT_PEER_PORT)
}

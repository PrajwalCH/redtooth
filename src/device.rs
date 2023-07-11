use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::interface;

const TCP_PORT: u16 = 25802;

pub type Id = u64;
pub type Address = SocketAddr;

pub struct Device {
    pub id: Id,
    pub address: Address,
}

impl Device {
    pub fn new(id: Id, address: Address) -> Self {
        Self { id, address }
    }

    /// Creates a new instance representing current device.
    pub fn current() -> Self {
        let address = Address::new(
            IpAddr::V4(interface::local_ipv4_address().unwrap_or(Ipv4Addr::UNSPECIFIED)),
            TCP_PORT,
        );
        let id = {
            let mut hasher = DefaultHasher::new();
            address.hash(&mut hasher);
            hasher.finish()
        };

        Self { id, address }
    }
}

use std::net::{IpAddr, Ipv4Addr};
use std::ptr;

pub fn local_ipv4_address() -> Option<IpAddr> {
    InterfaceAddresses::new()?.find(|ip_addr| {
        let IpAddr::V4(addr) = ip_addr else {
            return false;
        };
        addr.is_private() && addr.octets().starts_with(&[192, 168])
    })
}

struct InterfaceAddresses {
    /// A linked list containing interfaces of the system.
    interfaces: *mut libc::ifaddrs,
    next_interface: *mut libc::ifaddrs,
}

impl InterfaceAddresses {
    pub fn new() -> Option<Self> {
        let mut interfaces: *mut libc::ifaddrs = ptr::null_mut();

        unsafe {
            if libc::getifaddrs(&mut interfaces) == -1 {
                return None;
            }
        }
        Some(Self {
            interfaces,
            next_interface: interfaces,
        })
    }

    fn get_interface_ip_address(interface: *mut libc::ifaddrs) -> Option<IpAddr> {
        let interface_address = unsafe { *interface }.ifa_addr;
        let address_family = unsafe { *interface_address }.sa_family;

        match address_family as libc::c_int {
            libc::AF_INET => {
                // Cast `sockaddr` to `sockaddr_in` to get the address bytes `u32`.
                //
                // Raw bytes are needed to convert them into correct network byte order before
                // converting them into `Ipv4Addr`.
                let socket_address = interface_address as *mut libc::sockaddr_in;

                // Get the address bytes and convert them into network byte order by calling `to_be`.
                //
                // Without converting them into correct network byte order, the address would look
                // like this `1.1.168.192` instead of `192.168.1.1`.
                let address_bytes = unsafe { *socket_address }.sin_addr.s_addr.to_be();
                Some(IpAddr::V4(Ipv4Addr::from(address_bytes)))
            }
            // Not using ipv6 (currently) and packet.
            _ => None,
        }
    }
}

impl Iterator for InterfaceAddresses {
    type Item = IpAddr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_interface.is_null() {
            return None;
        }
        let current_interface = self.next_interface;
        self.next_interface = unsafe { *current_interface }.ifa_next;
        Self::get_interface_ip_address(current_interface).or_else(|| self.next())
    }
}

impl Drop for InterfaceAddresses {
    fn drop(&mut self) {
        unsafe {
            libc::freeifaddrs(self.interfaces);
        }
    }
}

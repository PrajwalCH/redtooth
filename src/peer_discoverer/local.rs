//! A local peer discoverer.

use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread::Builder as ThreadBuilder;

use super::{AnnouncementPkt, PeerMap, ThreadHandle};
use crate::{elogln, logln};

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;

/// Spawns a local server.
pub fn spawn(peer_map: Arc<Mutex<PeerMap>>) -> io::Result<ThreadHandle> {
    let builder = ThreadBuilder::new().name(String::from("local discovery"));
    builder.spawn(move || discover_peers(peer_map))
}

/// Announces the peer to other instances of the local server.
pub fn announce_peer(pkt: &[u8]) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    // Don't announce to the current instance of the server.
    socket.set_multicast_loop_v4(false)?;
    socket.send_to(pkt, (MULTICAST_ADDR, MULTICAST_PORT))?;
    Ok(())
}

/// Starts listening for an **announcement** a packet on the local network.
fn discover_peers(peer_map: Arc<Mutex<PeerMap>>) -> io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", MULTICAST_PORT))?;
    socket.join_multicast_v4(&MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)?;
    logln!("Listening for new announcement on {}", socket.local_addr()?);

    loop {
        let mut raw_pkt = [0; 4096];
        let Ok((pkt_len, announcement_addr)) = socket.recv_from(&mut raw_pkt) else {
            continue;
        };

        let Some(pkt) = AnnouncementPkt::from_bytes(&raw_pkt[..pkt_len]).ok() else {
            elogln!("Received a badly formatted packet from {announcement_addr}");
            continue;
        };
        let (Some(id), Some(mut addr)) = (pkt.get_peer_id(), pkt.get_peer_addr()) else {
            elogln!("Peer id and address are missing from the packet");
            continue
        };

        // If the address present in a packet is unspecified (0.0.0.0), use the address from
        // which the peer announces itself.
        if addr.ip().is_unspecified() {
            addr.set_ip(announcement_addr.ip());
        }

        // Unlock the map's lock ASAP using inner block.
        {
            let mut peer_map = match peer_map.try_lock() {
                Ok(guard) => guard,
                Err(TryLockError::Poisoned(p)) => p.into_inner(),
                Err(TryLockError::WouldBlock) => {
                    elogln!("Peer map's lock is currently acquired by some other component");
                    continue;
                }
            };
            peer_map.insert(id, addr);
        }
        logln!("Discovered `{addr}`");
    }
}

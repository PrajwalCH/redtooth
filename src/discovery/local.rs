//! A local peer discoverer.

use std::net::{Ipv4Addr, UdpSocket};
use std::sync::{Arc, Mutex, TryLockError};
use std::{io, thread};

use super::{Announcement, PeerMap, ThreadHandle};
use crate::{elogln, logln};

// Range between `224.0.0.0` to `224.0.0.250` is reserved or use by routing and maintenance
// protocols inside a network.
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 20581;

/// Spawns a local server.
pub fn spawn(peer_map: Arc<Mutex<PeerMap>>) -> io::Result<ThreadHandle> {
    thread::Builder::new()
        .name(String::from("local_discovery"))
        .spawn(move || discover_peers(peer_map))
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

        let mut announcement = match Announcement::from_bytes(&raw_pkt[..pkt_len]) {
            Ok(a) => a,
            Err(e) => {
                elogln!("Received a badly formatted packet; {e}");
                continue;
            }
        };
        // If the address present in a packet is unspecified (0.0.0.0), use the address from
        // which the peer announces itself.
        if announcement.peer_addr.ip().is_unspecified() {
            announcement.peer_addr.set_ip(announcement_addr.ip());
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
            peer_map.insert(announcement.peer_id, announcement.peer_addr);
        }
        logln!("Discovered `{}`", announcement.peer_addr);
    }
}

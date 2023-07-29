//! A local peer discoverer.

use std::io;
use std::net::{Ipv4Addr, UdpSocket};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread::Builder as ThreadBuilder;

use super::{PeerMap, ThreadHandle};
use crate::protocol::{PeerAddr, PeerID};
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
pub fn announce_peer(id: PeerID, addr: PeerAddr) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    // Don't announce to the current instance of the server.
    socket.set_multicast_loop_v4(false)?;

    let packet = format!("{id};{addr}");
    socket.send_to(packet.as_bytes(), (MULTICAST_ADDR, MULTICAST_PORT))?;
    Ok(())
}

/// Starts listening for an **announcement** a packet on the local network.
fn discover_peers(peer_map: Arc<Mutex<PeerMap>>) -> io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", MULTICAST_PORT))?;
    // socket.set_read_timeout(Some(Duration::from_millis(20)))?;
    socket.join_multicast_v4(&MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)?;
    logln!("Listening for new announcement on {}", socket.local_addr()?);

    loop {
        let mut packet = [0; 4096];
        let Ok((packet_len, announcement_addr)) = socket.recv_from(&mut packet) else {
            continue;
        };
        let Some((id, mut addr)) = parse_packet(&packet[..packet_len]) else {
            elogln!("Received a badly formatted packet from {announcement_addr}");
            continue;
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

/// Parses the packet and returns the id and address.
///
/// ## Panics
///
/// If the packet is not valid, UTF-8.
fn parse_packet(packet: &[u8]) -> Option<(PeerID, PeerAddr)> {
    let packet = String::from_utf8(packet.to_vec()).unwrap();
    let mut content_iter = packet.split(';');

    let id = content_iter.next()?.parse::<PeerID>().ok()?;
    let addr = content_iter.next()?.parse::<PeerAddr>().ok()?;
    Some((id, addr))
}

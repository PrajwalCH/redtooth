mod local;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{fmt, io};

use crate::protocol::packet::{InvalidHeaderSequence, Packet};
use crate::protocol::{PeerAddr, PeerID};

type PeerMap = HashMap<PeerID, PeerAddr>;
type ThreadHandle = JoinHandle<io::Result<()>>;

#[allow(dead_code)]
pub struct PeerDiscoverer {
    peers: Arc<Mutex<PeerMap>>,
    announcement_pkt: Vec<u8>,
    local_server_handle: Option<ThreadHandle>,
}

impl PeerDiscoverer {
    pub fn new(id: PeerID, addr: PeerAddr) -> PeerDiscoverer {
        Self {
            peers: Arc::new(Mutex::new(PeerMap::new())),
            announcement_pkt: Announcement::new(id, addr).as_bytes(),
            local_server_handle: None,
        }
    }

    /// Starts a server for discovering peers on either local or global or both networks.
    pub fn start(&mut self) -> io::Result<()> {
        let thread_handle = local::spawn(Arc::clone(&self.peers))?;
        self.local_server_handle = Some(thread_handle);
        Ok(())
    }

    /// Announces the peer to other instances of the server.
    pub fn announce_peer(&self) -> io::Result<()> {
        local::announce_peer(&self.announcement_pkt)
    }

    /// Returns the identifiers of all the discovered peers.
    pub fn get_discovered_peer_ids(&self) -> Option<Vec<PeerID>> {
        self.peers
            .lock()
            .ok()
            .and_then(|peer_map| (!peer_map.is_empty()).then(|| peer_map.keys().copied().collect()))
    }

    /// Returns a list of addresses for all the discovered peers.
    pub fn get_discovered_peer_addrs(&self) -> Option<Vec<PeerAddr>> {
        self.peers.lock().ok().and_then(|peer_map| {
            (!peer_map.is_empty()).then(|| peer_map.values().copied().collect())
        })
    }

    /// Returns the address of a specific peer that matches the given identifier.
    pub fn find_peer_addr_by_id(&self, id: PeerID) -> Option<PeerAddr> {
        self.peers
            .lock()
            .ok()
            .and_then(|peer_map| peer_map.get(&id).copied())
    }
}

enum InvalidAnnouncement {
    MissingPeerID,
    MissingPeerAddr,
    InvalidPacket(InvalidHeaderSequence),
}

impl fmt::Display for InvalidAnnouncement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::InvalidAnnouncement::*;

        match self {
            MissingPeerID => write!(f, "missing peer id"),
            MissingPeerAddr => write!(f, "missing peer address"),
            InvalidPacket(e) => write!(f, "invalid packet: {e}"),
        }
    }
}

struct Announcement {
    peer_id: PeerID,
    peer_addr: PeerAddr,
}

impl Announcement {
    pub fn new(peer_id: PeerID, peer_addr: PeerAddr) -> Announcement {
        Announcement { peer_id, peer_addr }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Announcement, InvalidAnnouncement> {
        let packet = Packet::from_bytes(bytes).map_err(InvalidAnnouncement::InvalidPacket)?;

        let peer_id = packet
            .get_header("id")
            .and_then(|id| id.parse::<PeerID>().ok())
            .ok_or(InvalidAnnouncement::MissingPeerID)?;
        let peer_addr = packet
            .get_header("addr")
            .and_then(|addr| addr.parse::<PeerAddr>().ok())
            .ok_or(InvalidAnnouncement::MissingPeerAddr)?;

        Ok(Announcement { peer_id, peer_addr })
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut packet = Packet::new();
        packet.set_header("id", self.peer_id);
        packet.set_header("addr", self.peer_addr);
        packet.as_bytes()
    }
}

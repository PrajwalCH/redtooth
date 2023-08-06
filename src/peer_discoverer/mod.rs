mod local;

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::protocol::PeerAddr;
use crate::protocol::PeerID;
use crate::protocol::{Packet, PacketParseError};

type PeerMap = HashMap<PeerID, PeerAddr>;
type ThreadHandle = JoinHandle<io::Result<()>>;

#[allow(dead_code)]
pub struct PeerDiscoverer {
    peers: Arc<Mutex<PeerMap>>,
    announcement_pkt: AnnouncementPkt<'static>,
    local_server_handle: Option<ThreadHandle>,
}

impl PeerDiscoverer {
    pub fn new(id: PeerID, addr: PeerAddr) -> PeerDiscoverer {
        Self {
            peers: Arc::new(Mutex::new(PeerMap::new())),
            announcement_pkt: AnnouncementPkt::new(id, addr),
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
        local::announce_peer(&self.announcement_pkt.as_bytes())
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

struct AnnouncementPkt<'a>(Packet<'a>);

impl<'a> AnnouncementPkt<'a> {
    pub fn new(id: PeerID, addr: PeerAddr) -> AnnouncementPkt<'a> {
        let mut pkt = Packet::new();
        pkt.set_header("id", id);
        pkt.set_header("addr", addr);

        AnnouncementPkt(pkt)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<AnnouncementPkt, PacketParseError> {
        Ok(AnnouncementPkt(Packet::from_bytes(bytes)?))
    }

    pub fn get_peer_id(&self) -> Option<PeerID> {
        self.0
            .get_header("id")
            .and_then(|id| id.parse::<PeerID>().ok())
    }

    pub fn get_peer_addr(&self) -> Option<PeerAddr> {
        self.0
            .get_header("addr")
            .and_then(|addr| addr.parse::<PeerAddr>().ok())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_bytes()
    }
}

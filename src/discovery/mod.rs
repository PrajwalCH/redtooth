mod announcement;
mod local;

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use self::announcement::Announcement;
use crate::protocol::{PeerAddr, PeerID};

type PeerMap = HashMap<PeerID, PeerAddr>;
type ThreadHandle = JoinHandle<io::Result<()>>;

pub struct PeerDiscoverer {
    peers: Arc<Mutex<PeerMap>>,
    announcement_pkt: Vec<u8>,
}

impl PeerDiscoverer {
    pub fn new(id: PeerID, addr: PeerAddr) -> PeerDiscoverer {
        Self {
            peers: Arc::new(Mutex::new(PeerMap::new())),
            announcement_pkt: Announcement::new(id, addr).as_bytes(),
        }
    }

    /// Spawns a server for discovering peers on either local or global or both networks.
    pub fn spawn(&mut self) -> io::Result<()> {
        local::spawn(Arc::clone(&self.peers))?;
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

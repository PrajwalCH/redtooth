mod local;

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::protocol::PeerAddr;
use crate::protocol::PeerID;

type PeerMap = HashMap<PeerID, PeerAddr>;
type ThreadHandle = JoinHandle<io::Result<()>>;

#[allow(dead_code)]
pub struct DiscoveryServer {
    peers: Arc<Mutex<PeerMap>>,
    local_server_handle: Option<ThreadHandle>,
}

impl DiscoveryServer {
    pub fn new() -> DiscoveryServer {
        Self {
            peers: Arc::new(Mutex::new(PeerMap::new())),
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
    pub fn announce_peer(&self, id: PeerID, addr: PeerAddr) -> io::Result<()> {
        local::announce_peer(id, addr)
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

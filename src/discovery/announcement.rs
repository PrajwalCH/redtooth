use std::fmt;

use crate::protocol::packet::{InvalidHeaderSequence, Packet};
use crate::protocol::{PeerAddr, PeerID};

pub enum InvalidAnnouncement {
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

pub struct Announcement {
    pub peer_id: PeerID,
    pub peer_addr: PeerAddr,
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

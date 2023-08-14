use std::io::{self, Read, Write};
use std::os::unix::net::{Incoming, UnixListener, UnixStream};

use crate::protocol::packet::Packet;
use crate::protocol::PeerID;

pub const SOCK_FILE_PATH: &str = "/tmp/rapi.sock";

/// Represents a command sent to IPC server.
pub enum Command {
    MyID,
    MyAddr,
    DiscoveredPeers,
    Send(String),
    SendTo(PeerID, String),
}

impl TryFrom<Packet<'_>> for Command {
    type Error = ();

    fn try_from(p: Packet) -> Result<Self, Self::Error> {
        let command = match p.get_header("command").ok_or(())? {
            "myid" => Command::MyID,
            "myaddr" => Command::MyAddr,
            "discovered_peers" => Command::DiscoveredPeers,
            "send" => {
                let file_name = p.get_header("args").ok_or(())?.to_string();
                Command::Send(file_name)
            }
            "send_to" => {
                let args = p.get_header("args").ok_or(())?.split_once(' ').ok_or(())?;
                let peer_id = args.0.parse::<PeerID>().map_err(|_| ())?;
                let file_name = args.1.to_string();
                Command::SendTo(peer_id, file_name)
            }
            _ => return Err(()),
        };
        Ok(command)
    }
}

/// A structure representing an IPC socket server.
pub struct IPCListener(UnixListener);

impl IPCListener {
    /// Creates a new [IPCListener] bound to the [`SOCK_FILE_PATH`].
    pub fn new() -> IPCListener {
        let listener = UnixListener::bind(SOCK_FILE_PATH).unwrap();
        IPCListener(listener)
    }

    /// Returns an iterator over incoming messages.
    pub fn incoming_message(&self) -> IncomingMessage {
        IncomingMessage(self.0.incoming())
    }
}

/// An iterator over incoming messages to a [`IPCListener`].
///
/// It will never return None.
pub struct IncomingMessage<'l>(Incoming<'l>);

impl<'l> Iterator for IncomingMessage<'l> {
    type Item = io::Result<Message>;

    fn next(&mut self) -> Option<io::Result<Message>> {
        let mut stream = match self.0.next()? {
            Ok(s) => s,
            Err(e) => return Some(Err(e)),
        };
        let mut raw_bytes = Vec::new();

        if let Err(e) = stream.read_to_end(&mut raw_bytes) {
            return Some(Err(e));
        }
        // Ignore the invalid packet.
        let packet = Packet::from_bytes(&raw_bytes).ok()?;
        let command = Command::try_from(packet).ok()?;
        Some(Ok(Message::new(command, stream)))
    }
}

/// Represents a message sent to IPC server.
pub struct Message {
    command: Command,
    stream: UnixStream,
}

impl Message {
    pub fn new(command: Command, stream: UnixStream) -> Message {
        Message { command, stream }
    }

    pub fn response<D>(&mut self, data: D) -> io::Result<()>
    where
        D: AsRef<[u8]>,
    {
        self.stream.write_all(data.as_ref())
    }
}

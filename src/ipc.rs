use std::io::{self, Read, Write};
use std::os::unix::net::{Incoming, UnixListener, UnixStream};
use std::str::FromStr;

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

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (cmd, args) = s
            .strip_prefix('/')
            .and_then(|v| v.split_once(' ').or(Some((v, ""))))
            .ok_or(())?;

        match cmd {
            "myid" => Ok(Command::MyID),
            "myaddr" => Ok(Command::MyAddr),
            "discovered_peers" => Ok(Command::DiscoveredPeers),
            "send" => Ok(Command::Send(args.to_string())),
            "send_to" => {
                let args = args.split_once(' ').ok_or(())?;
                let peer_id = args.0.parse::<PeerID>().map_err(|_| ())?;
                let file_name = args.1.to_string();
                Ok(Command::SendTo(peer_id, file_name))
            }
            _ => Err(()),
        }
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
        let mut request = String::new();

        if let Err(e) = stream.read_to_string(&mut request) {
            return Some(Err(e));
        }
        let command = Command::from_str(&request).ok()?;
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

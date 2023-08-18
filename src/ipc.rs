use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::str::FromStr;

use crate::protocol::PeerID;

pub const SOCK_FILE_PATH: &str = "/tmp/rapi.sock";

/// A structure representing an IPC socket server.
pub struct IPCServer(UnixListener);

impl IPCServer {
    /// Creates a new [IPCServer] bound to the [`SOCK_FILE_PATH`].
    pub fn new() -> io::Result<IPCServer> {
        Ok(IPCServer(UnixListener::bind(SOCK_FILE_PATH)?))
    }

    /// Returns an iterator over incoming messages.
    pub fn incoming_messages(&self) -> IncomingMessages {
        IncomingMessages { server: self }
    }

    /// Accepts a new incoming connection and returns a new message read from it.
    ///
    /// This function will block the calling thread until a new connection is established.
    /// When established, it reads the message and returns it.
    fn recv_message(&self) -> io::Result<Message> {
        let mut stream = self.0.accept().map(|(stream, _)| stream)?;
        let mut request = String::new();
        stream.read_to_string(&mut request)?;

        match Command::from_str(&request) {
            Ok(c) => Ok(Message::new(c, stream)),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid command")),
        }
    }
}

/// An iterator over incoming messages to a [`IPCServer`].
///
/// It will never return None.
pub struct IncomingMessages<'s> {
    server: &'s IPCServer,
}

impl<'s> Iterator for IncomingMessages<'s> {
    type Item = Message;

    fn next(&mut self) -> Option<Message> {
        self.server.recv_message().ok()
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

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::MyID => write!(f, "/myid"),
            Command::MyAddr => write!(f, "/myaddr"),
            Command::DiscoveredPeers => write!(f, "/discovered_peers"),
            Command::Send(file_name) => write!(f, "/send {file_name}"),
            Command::SendTo(peer_id, file_name) => write!(f, "/send_to {peer_id} {file_name}"),
        }
    }
}

use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

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
        Ok(Message::new(request, stream))
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
    request: String,
    response_stream: UnixStream,
}

impl Message {
    pub fn new(request: String, response_stream: UnixStream) -> Message {
        Message {
            request,
            response_stream,
        }
    }

    pub fn command(&self) -> Option<Command> {
        let (cmd, args) = self
            .request
            .strip_prefix('/')
            .and_then(|v| v.split_once(' ').or(Some((v, ""))))?;

        match cmd {
            "myid" => Some(Command::MyID),
            "myaddr" => Some(Command::MyAddr),
            "peers" => Some(Command::Peers),
            "send" => Some(Command::Send(args.to_string())),
            "send_to" => {
                let args = args.split_once(' ')?;
                let peer_id = args.0.parse::<PeerID>().ok()?;
                let file_name = args.1.to_string();
                Some(Command::SendTo(peer_id, file_name))
            }
            _ => None,
        }
    }

    pub fn response(&mut self, data: impl fmt::Display) -> io::Result<()> {
        write!(self.response_stream, "{data}")
    }
}

/// Represents a command sent to IPC server.
pub enum Command {
    MyID,
    MyAddr,
    Peers,
    Send(String),
    SendTo(PeerID, String),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::MyID => write!(f, "/myid"),
            Command::MyAddr => write!(f, "/myaddr"),
            Command::Peers => write!(f, "/peers"),
            Command::Send(file_name) => write!(f, "/send {file_name}"),
            Command::SendTo(peer_id, file_name) => write!(f, "/send_to {peer_id} {file_name}"),
        }
    }
}

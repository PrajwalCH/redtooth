use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

use crate::api::{Command, Message, ReadMessage};
use crate::protocol::PeerID;

pub const SOCK_FILE_PATH: &str = "/tmp/rapi.sock";

/// A structure representing an IPC socket server.
pub struct IPCServer(UnixListener);

impl IPCServer {
    /// Creates a new [IPCServer] bound to the [`SOCK_FILE_PATH`].
    pub fn new() -> io::Result<IPCServer> {
        Ok(IPCServer(UnixListener::bind(SOCK_FILE_PATH)?))
    }
}

impl ReadMessage for IPCServer {
    /// Accepts a new incoming connection and returns a new message read from it.
    ///
    /// This function will block the calling thread until a new connection is established.
    /// When established, it reads the message and returns it.
    fn read_message(&self) -> io::Result<Message> {
        let mut stream = self.0.accept().map(|(stream, _)| stream)?;
        let mut request = String::new();
        stream.read_to_string(&mut request)?;

        let command = parse_request(&request)
            .ok_or(io::Error::new(io::ErrorKind::Other, "invalid command"))?;
        Ok(Message::new(command, Box::new(stream)))
    }
}

fn parse_request(req: &str) -> Option<Command> {
    let (cmd, args) = req
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

pub fn send_request(c: Command) -> io::Result<String> {
    let mut stream = UnixStream::connect(SOCK_FILE_PATH)?;

    match c {
        Command::MyID => write!(stream, "/myid")?,
        Command::MyAddr => write!(stream, "/myaddr")?,
        Command::Peers => write!(stream, "/peers")?,
        Command::Send(file_name) => write!(stream, "/send {file_name}")?,
        Command::SendTo(peer_id, file_name) => write!(stream, "/send_to {peer_id} {file_name}")?,
    };
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

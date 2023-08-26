use std::fs;
use std::io::{self, Error, ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

use crate::api::{Message, ReadRequest, Request};
use crate::protocol::PeerID;

pub const SOCK_FILE_PATH: &str = "/tmp/rapi.sock";

/// A structure representing an IPC socket server.
pub struct IPCServer(UnixListener);

impl IPCServer {
    /// Creates a new [IPCServer] bound to the [`SOCK_FILE_PATH`].
    pub fn new() -> io::Result<IPCServer> {
        let listener = UnixListener::bind(SOCK_FILE_PATH);

        if !listener
            .as_ref()
            .is_err_and(|e| e.kind() == ErrorKind::AddrInUse)
        {
            Ok(IPCServer(listener?))
        } else {
            // Delete the old socket file and create the new one.
            fs::remove_file(SOCK_FILE_PATH)?;
            Ok(IPCServer(UnixListener::bind(SOCK_FILE_PATH)?))
        }
    }
}

impl ReadRequest for IPCServer {
    /// Accepts a new incoming connection and returns a new request received from it.
    ///
    /// This function will block the calling thread until a new connection is established.
    /// When established, it reads the request and returns it.
    fn read_request(&self) -> io::Result<Request> {
        let mut stream = self.0.accept().map(|(stream, _)| stream)?;
        let mut request = String::new();
        stream.read_to_string(&mut request)?;

        let command =
            parse_request(&request).ok_or(Error::new(ErrorKind::Other, "invalid command"))?;
        Ok(Request::new(command, Box::new(stream)))
    }
}

fn parse_request(req: &str) -> Option<Message> {
    let (cmd, args) = req
        .strip_prefix('/')
        .and_then(|v| v.split_once(' ').or(Some((v, ""))))?;

    match cmd {
        "myid" => Some(Message::MyID),
        "myaddr" => Some(Message::MyAddr),
        "peers" => Some(Message::Peers),
        "send" => Some(Message::Send(args.to_string())),
        "send_to" => {
            let args = args.split_once(' ')?;
            let peer_id = args.0.parse::<PeerID>().ok()?;
            let file_name = args.1.to_string();
            Some(Message::SendTo(peer_id, file_name))
        }
        _ => None,
    }
}

pub fn send_request(msg: Message) -> io::Result<String> {
    let mut stream = UnixStream::connect(SOCK_FILE_PATH)?;

    match msg {
        Message::MyID => write!(stream, "/myid")?,
        Message::MyAddr => write!(stream, "/myaddr")?,
        Message::Peers => write!(stream, "/peers")?,
        Message::Send(file_name) => write!(stream, "/send {file_name}")?,
        Message::SendTo(peer_id, file_name) => write!(stream, "/send_to {peer_id} {file_name}")?,
    };
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

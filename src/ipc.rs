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

/// A structure representing an IPC socket server.
pub struct IpcListener(UnixListener);

impl IpcListener {
    /// Creates a new [IpcListener] bound to the [`SOCK_FILE_PATH`].
    pub fn new() -> IpcListener {
        let listener = UnixListener::bind(SOCK_FILE_PATH).unwrap();
        IpcListener(listener)
    }

    /// Returns an iterator over incoming messages.
    pub fn incoming_message(&self) -> IncomingMessage {
        IncomingMessage(self.0.incoming())
    }
}

/// An iterator over incoming messages to a [`IpcListener`].
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
        let _packet = Packet::from_bytes(&raw_bytes).ok()?;
        Some(Ok(Message::new(Command::MyID, stream)))
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

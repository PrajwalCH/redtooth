use std::fmt;
use std::io::{self, Write};

use crate::protocol::PeerID;

/// The `ReadMessage` trait allows for reading a message from a connection.
///
/// Implementors of the `ReadMessage` trait are called 'message readers'
/// and are defined by one required method, [`read_message()`].
///
/// Each call to [`read_message()`] will attempt to read and return a message
/// from a connection.
///
/// [`read_message()`]: ReadMessage::read_message
pub trait ReadMessage {
    /// Attempts to read and return a new message from a connection.
    ///
    /// This function may or may not block the calling thread while waiting
    /// for a connection to be established. When established, it reads a message
    /// and returns it.
    fn read_message(&self) -> io::Result<Request>;
}

/// Represents a command sent to API.
pub enum Command {
    MyID,
    MyAddr,
    Peers,
    Send(String),
    SendTo(PeerID, String),
}

/// The `Api` structure allows for creating a different kind of APIs (e.g., IPC, HTTP, etc.).
///
/// By providing a message reader that implements the [`ReadMessage`] trait,
/// the `Api` can fetch incoming messages from a specific connection or source.
pub struct Api<R> {
    message_reader: R,
}

impl<R: ReadMessage> Api<R> {
    /// Creates a new [`Api`] with the given message reader from where
    /// it can read a message.
    pub fn new(message_reader: R) -> Api<R> {
        Api { message_reader }
    }

    /// Returns an iterator over incoming messages.
    pub fn incoming_messages(&self) -> IncomingMessages<R> {
        IncomingMessages { api: self }
    }

    /// Receives the message from the given reader and returns it.
    fn recv_message(&self) -> io::Result<Request> {
        self.message_reader.read_message()
    }
}

/// An iterator over incoming messages to an [`Api`].
pub struct IncomingMessages<'a, R> {
    api: &'a Api<R>,
}

impl<'a, R: ReadMessage> Iterator for IncomingMessages<'a, R> {
    type Item = Request;

    fn next(&mut self) -> Option<Request> {
        self.api.recv_message().ok()
    }
}

/// Represents a request sent to an API.
pub struct Request {
    command: Command,
    response_writer: Box<dyn Write>,
}

impl Request {
    /// Creates a new instance of a request.
    pub fn new(command: Command, response_writer: Box<dyn Write>) -> Request {
        Request {
            command,
            response_writer,
        }
    }

    /// Returns a command attached in a request.
    pub fn command(&self) -> &Command {
        &self.command
    }

    /// Sends a response to this request.
    pub fn response(&mut self, data: impl fmt::Display) -> io::Result<()> {
        write!(self.response_writer, "{data}")
    }
}

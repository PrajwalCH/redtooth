use std::fmt;
use std::io::{self, Write};

use crate::protocol::PeerID;

/// The `ReadRequest` trait allows for reading a request from a connection.
///
/// Implementors of the `ReadRequest` trait are called 'request readers'
/// and are defined by one required method, [`read_request()`].
///
/// Each call to [`read_request()`] will attempt to read and return a request
/// from a connection.
///
/// [`read_request()`]: ReadRequest::read_request
pub trait ReadRequest {
    /// Attempts to read and return a new request from a connection.
    ///
    /// This function may or may not block the calling thread while waiting
    /// for a connection to be established.
    fn read_request(&self) -> io::Result<Request>;
}

/// Represents a message sent to API.
pub enum Message {
    MyID,
    MyAddr,
    Peers,
    Send(String),
    SendTo(PeerID, String),
}

/// The `Api` structure allows for creating a different kind of APIs (e.g., IPC, HTTP, etc.).
///
/// By providing a request reader that implements the [`ReadRequest`] trait,
/// the `Api` can fetch incoming requests from a specific connection or source.
pub struct Api<R> {
    request_reader: R,
}

impl<R: ReadRequest> Api<R> {
    /// Creates a new [`Api`] with the given request reader from where
    /// it can read a request.
    pub fn new(request_reader: R) -> Api<R> {
        Api { request_reader }
    }

    /// Returns an iterator over incoming requests.
    pub fn incoming_requests(&self) -> IncomingRequests<R> {
        IncomingRequests { api: self }
    }

    /// Receives the request from the given reader and returns it.
    fn recv_request(&self) -> io::Result<Request> {
        self.request_reader.read_request()
    }
}

/// An iterator over incoming requests to an [`Api`].
pub struct IncomingRequests<'a, R> {
    api: &'a Api<R>,
}

impl<'a, R: ReadRequest> Iterator for IncomingRequests<'a, R> {
    type Item = Request;

    fn next(&mut self) -> Option<Request> {
        self.api.recv_request().ok()
    }
}

/// Represents a request sent to an API.
pub struct Request {
    message: Message,
    response_writer: Box<dyn Write>,
}

impl Request {
    /// Creates a new instance of a request.
    pub fn new(message: Message, response_writer: Box<dyn Write>) -> Request {
        Request {
            message,
            response_writer,
        }
    }

    /// Returns a message attached in a request.
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Sends a response to this request.
    pub fn response(&mut self, data: impl fmt::Display) -> io::Result<()> {
        write!(self.response_writer, "{data}")
    }
}

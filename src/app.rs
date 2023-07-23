use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::Builder as ThreadBuilder;

use crate::device::{self, DeviceAddress, DeviceID};
use crate::discovery_server;
use crate::{elogln, logln};

pub struct App {
    device_id: DeviceID,
    device_address: DeviceAddress,
    event_channel: EventChannel,
    discovered_devices: HashMap<DeviceID, DeviceAddress>,
    /// Path where received file will be saved.
    save_location: PathBuf,
}

impl App {
    /// Creates a new instance of `App` with all the necessary setup.
    pub fn new() -> App {
        let home_env_key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let home_path = env::var(home_env_key).expect(
            "env variable `HOME` for linux and `USERPROFILE` for windows should be available",
        );

        App {
            device_id: device::id(),
            device_address: device::address(),
            event_channel: EventChannel::new(),
            discovered_devices: HashMap::new(),
            save_location: PathBuf::from(home_path),
        }
    }

    /// Returns the [`EventEmitter`] that can be used to send events to application's event loop.
    pub fn event_emitter(&self) -> EventEmitter {
        EventEmitter(self.event_channel.sender.clone())
    }

    /// Starts the main event loop.
    ///
    /// **NOTE:** This function always blocks the current thread.
    pub fn run(&mut self) -> io::Result<()> {
        self.start_data_receiver()?;
        discovery_server::start(self.event_emitter())?;
        discovery_server::announce_device(self.device_id, self.device_address)?;

        while let Ok(event) = self.event_channel.receiver.recv() {
            match event {
                Event::DataReceived(header, contents) => {
                    if let Err(e) = self.write_data(header, contents) {
                        elogln!("Encountered an error while writing data to the disk: {e}");
                    }
                }
                Event::NewDeviceDiscovered((id, address)) => {
                    self.discovered_devices.insert(id, address);
                }
            };
        }
        Ok(())
    }

    /// Starts a TCP server for receiving data.
    fn start_data_receiver(&self) -> io::Result<()> {
        let event_emitter = self.event_emitter();
        let listener = TcpListener::bind(self.device_address)?;
        let builder = ThreadBuilder::new().name(String::from("data receiver"));
        logln!("Receiving data on {}", listener.local_addr()?);

        builder.spawn(move || {
            for peer_stream in listener.incoming() {
                let Ok(mut peer_stream) = peer_stream else {
                    continue;
                };
                // Data should be in the following format:
                // ```
                // file_name: filename.jpeg
                // ::
                // file contents
                // ```
                let mut data: Vec<u8> = Vec::new();

                if let Err(e) = peer_stream.read_to_end(&mut data) {
                    elogln!("Failed to read received data: {e}");
                    continue;
                }
                // FIXME: The original data will be changed if we replace invalid UTF-8 sequences
                //        with replacement character `�`. Since image, binary and other
                //        non-textual files can contain invalid UTF-8 character, the current
                //        implementation doesn't handle them properly.
                let data = String::from_utf8_lossy(&data);
                let mut sections = data.split("::");

                match sections.next().map(DataHeader::from_str) {
                    Some(Ok(header)) => {
                        let contents = Vec::from(sections.next().unwrap_or_default());
                        event_emitter.emit(Event::DataReceived(header, contents));
                    }
                    Some(Err(e)) => {
                        elogln!("Unable to parse the header of received data: {e}");
                        continue;
                    }
                    None => {
                        elogln!("Received data does not contain the header");
                        continue;
                    }
                };
            }
        })?;
        Ok(())
    }

    /// Creates a file based on the provided header.
    fn write_data(&self, header: DataHeader, contents: Vec<u8>) -> io::Result<()> {
        let file_path = self.save_location.join(header.file_name);
        fs::write(file_path, contents)
    }
}

#[derive(Debug)]
pub enum Event {
    DataReceived(DataHeader, Vec<u8>),
    NewDeviceDiscovered((DeviceID, DeviceAddress)),
}

#[derive(Clone)]
pub struct EventEmitter(Sender<Event>);

impl EventEmitter {
    pub fn emit(&self, event: Event) {
        if let Err(SendError(event)) = self.0.send(event) {
            elogln!("Failed to emit `{event:?}` due to listener being disconnected");
        }
    }
}

struct EventChannel {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
}

impl EventChannel {
    pub fn new() -> EventChannel {
        let (sender, receiver) = mpsc::channel::<Event>();
        EventChannel { sender, receiver }
    }
}

/// An error returned from [`DataHeader::from_str`].
pub enum DataHeaderParseError {
    MissingName,
}

impl fmt::Display for DataHeaderParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::DataHeaderParseError::*;

        match self {
            MissingName => write!(f, "Missing required `name` field"),
        }
    }
}

/// Represents the header information for data being transmitted.
///
/// It encapsulates essential metadata about the file being sent.
/// This header is pre-pended to the actual file data before transmission,
/// allowing the receiver to correctly handle the incoming data.
#[derive(Debug)]
pub struct DataHeader {
    /// The name of the file, including its extension.
    pub(crate) file_name: String,
}

impl FromStr for DataHeader {
    type Err = DataHeaderParseError;

    fn from_str(s: &str) -> Result<DataHeader, DataHeaderParseError> {
        let name = s
            .trim()
            .strip_prefix("file_name: ")
            .ok_or(DataHeaderParseError::MissingName)?
            .to_string();

        Ok(Self { file_name: name })
    }
}

impl fmt::Display for DataHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "file_name: {}", self.file_name)
    }
}

use std::env;
use std::fs;
use std::io::{self, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::Builder as ThreadBuilder;

use crate::discovery_server::DiscoveryServer;
use crate::protocol::{self, DeviceAddress, DeviceID};
use crate::protocol::{DataHeader, DATA_SECTIONS_SEPARATOR};
use crate::{elogln, logln};

pub struct App {
    device_id: DeviceID,
    device_address: DeviceAddress,
    event_channel: EventChannel,
    discovery_server: DiscoveryServer,
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
            device_id: protocol::device_id(),
            device_address: protocol::device_address(),
            event_channel: EventChannel::new(),
            discovery_server: DiscoveryServer::new(),
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
        self.discovery_server.start()?;
        self.discovery_server
            .announce_device(self.device_id, self.device_address)?;

        while let Ok(event) = self.event_channel.receiver.recv() {
            match event {
                Event::DataReceived(header, contents) => {
                    if let Err(e) = self.write_data(header, contents) {
                        elogln!("Encountered an error while writing data to the disk: {e}");
                    }
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

                let separator_len = DATA_SECTIONS_SEPARATOR.len();
                let Some(separator_index) = data
                    .windows(separator_len)
                    .position(|bytes| bytes == DATA_SECTIONS_SEPARATOR) else
                {
                    elogln!("Data sections separator are missing from the received data");
                    continue;
                };
                let raw_header = std::str::from_utf8(&data[..separator_index]).unwrap_or_default();

                match DataHeader::from_str(raw_header) {
                    Ok(header) => {
                        // Skip all the separator bytes.
                        let file_contents = data.get(separator_index + separator_len..);
                        // If a valid header and separator are present but the contents are missing,
                        // declare it as a empty.
                        let file_contents = file_contents.unwrap_or_default().to_owned();
                        event_emitter.emit(Event::DataReceived(header, file_contents));
                    }
                    Err(e) => {
                        elogln!("Unable to parse the header of received data: {e}");
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

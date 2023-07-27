use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::Builder as ThreadBuilder;

use crate::discovery_server::DiscoveryServer;
use crate::elogln;
use crate::protocol::{self, DeviceAddress, DeviceID, FilePacket};
use crate::receiver;

#[derive(Debug)]
pub enum Event {
    FileReceived(FilePacket),
}

pub struct App {
    device_id: DeviceID,
    device_address: DeviceAddress,
    event_channel: EventChannel,
    discovery_server: DiscoveryServer,
    /// Path where the received file will be saved.
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

        // TODO: Implement either ipc or http api server so that both cli and web ui can talk.
        loop {}
    }

    /// Starts a TCP server for receiving data.
    fn start_data_receiver(&self) -> io::Result<()> {
        let receiving_addr = self.device_address;
        let save_location = self.save_location.clone();
        let builder = ThreadBuilder::new().name(String::from("data_receiver"));
        builder.spawn(move || receiver::start_file_receiving(receiving_addr, save_location))?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct EventEmitter(Sender<Event>);

impl EventEmitter {
    pub fn emit(&self, event: Event) {
        if let Err(SendError(event)) = self.0.send(event) {
            elogln!("Failed to emit event `{event:?}`: all the listeners are disconnected");
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

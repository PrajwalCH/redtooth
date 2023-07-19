use std::collections::HashMap;
use std::io::{self, Read};
use std::net::TcpListener;
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
}

impl App {
    /// Creates a new instance of `App` with all the necessary setup.
    pub fn new() -> App {
        App {
            device_id: device::id(),
            device_address: device::address(),
            event_channel: EventChannel::new(),
            discovered_devices: HashMap::new(),
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
                Event::DataReceived(data) => {
                    self.handle_data(data);
                }
                Event::NewDeviceDiscovered((id, address)) => {
                    self.discovered_devices.insert(id, address);
                }
                Event::PingAll => {
                    // for device_address in self.discovered_devices.values() {
                    //     let mut device_stream = TcpStream::connect(device_address).unwrap();
                    //     device_stream.write_all("ping".as_bytes()).unwrap();
                    // }
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
                let mut buffer: Vec<u8> = Vec::new();

                if let Err(e) = peer_stream.read_to_end(&mut buffer) {
                    elogln!("Failed to read received data: {e}");
                    continue;
                }
                event_emitter.emit(Event::DataReceived(buffer));
            }
        })?;
        Ok(())
    }

    fn handle_data(&self, data: Vec<u8>) {
        todo!()
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

#[derive(Debug)]
pub enum Event {
    DataReceived(Vec<u8>),
    NewDeviceDiscovered((DeviceID, DeviceAddress)),
    /// Sends a ping message to all the devices.
    PingAll,
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

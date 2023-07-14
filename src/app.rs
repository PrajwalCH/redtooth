use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, RecvError, SendError, Sender};
use std::thread::Builder as ThreadBuilder;

use crate::device::{self, DeviceAddress, DeviceID};
use crate::discovery_server;

pub struct App {
    device_id: DeviceID,
    device_address: DeviceAddress,
    event_emitter: EventEmitter,
    event_listener: EventListener,
    discovered_devices: HashMap<DeviceID, DeviceAddress>,
}

impl App {
    /// Creates a new instance representing current device.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<Event>();

        Self {
            device_id: device::id(),
            device_address: device::address(),
            event_emitter: EventEmitter::new(sender),
            event_listener: EventListener::new(receiver),
            discovered_devices: HashMap::new(),
        }
    }

    /// Starts the main event loop.
    pub fn run(mut self) -> io::Result<()> {
        self.start_data_receiver()?;
        discovery_server::announce_device(self.device_id, self.device_address)?;
        discovery_server::start_local_discovery(self.event_emitter())?;

        let builder = ThreadBuilder::new().name(String::from("event loop"));

        builder.spawn(move || loop {
            let Ok(event) = self.event_listener.listen() else {
                    continue;
                };

            match event {
                Event::DiscoveredNewDevice((id, address)) => {
                    self.discovered_devices.insert(id, address);
                }
                Event::PingAll => {
                    for device_address in self.discovered_devices.values() {
                        let mut device_stream = TcpStream::connect(device_address).unwrap();
                        device_stream.write_all("ping".as_bytes()).unwrap();
                    }
                }
            };
        })?;
        Ok(())
    }

    pub fn event_emitter(&self) -> EventEmitter {
        self.event_emitter.clone()
    }

    /// Starts a TCP server for receiving data.
    pub fn start_data_receiver(&self) -> io::Result<()> {
        let builder = ThreadBuilder::new().name(String::from("data receiver"));
        let listener = TcpListener::bind(self.device_address)?;
        println!("[Me]: Receiving data on: {}", listener.local_addr()?);

        builder.spawn(move || {
            for peer_stream in listener.incoming() {
                let Ok(mut peer_stream) = peer_stream else {
                    continue;
                };

                // NOTE: For now the buffer is only used for holding `ping` message.
                let mut data_buffer = [0; 6];
                peer_stream.read_exact(&mut data_buffer).ok();

                let data = std::str::from_utf8(&data_buffer).unwrap();
                let peer_address = peer_stream.peer_addr().unwrap();

                if !data_buffer.is_empty() {
                    println!("[Main]: Received `{data}` from {peer_address}");
                }
            }
        })?;
        Ok(())
    }
}

pub enum Event {
    DiscoveredNewDevice((DeviceID, DeviceAddress)),
    /// Send ping message to all the devices.
    PingAll,
}

#[derive(Clone)]
pub struct EventEmitter {
    sender: Sender<Event>,
}

impl EventEmitter {
    pub fn new(sender: Sender<Event>) -> Self {
        Self { sender }
    }

    pub fn emit(&self, event: Event) -> Result<(), SendError<Event>> {
        self.sender.send(event)
    }
}

pub struct EventListener {
    receiver: Receiver<Event>,
}

impl EventListener {
    pub fn new(receiver: Receiver<Event>) -> Self {
        Self { receiver }
    }

    pub fn listen(&self) -> Result<Event, RecvError> {
        self.receiver.recv()
    }

    // pub fn try_listen(&self) -> Result<Event, TryRecvError> {
    //     self.receiver.try_recv()
    // }
}

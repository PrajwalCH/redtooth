use std::io::{self, Read};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::Builder as ThreadBuilder;

use crate::device::{self, DeviceAddress, DeviceID};
use crate::discovery_server::DiscoveryServer;
use crate::{elogln, logln};

#[allow(dead_code)]
pub struct App {
    device_id: DeviceID,
    device_address: DeviceAddress,
    event_channel: EventChannel,
    discovery_server: DiscoveryServer,
}

impl App {
    /// Creates a new instance of `App` with all the necessary setup.
    pub fn new() -> App {
        let device_id = device::id();
        let device_address = device::address();

        App {
            device_id,
            device_address,
            event_channel: EventChannel::new(),
            discovery_server: DiscoveryServer::new(device_id, device_address),
        }
    }

    /// Starts the main event loop.
    ///
    /// **NOTE:** This function always blocks the current thread.
    pub fn run(&mut self) -> io::Result<()> {
        self.start_data_receiver()?;
        self.discovery_server.start()?;
        self.discovery_server.announce_device()?;

        loop {
            // SAFETY: Event receiving can only fail if all event senders are disconnected, which is
            // not possible since we have at least one sender.
            let event = self.event_channel.receiver.recv().unwrap();

            match event {
                Event::PingAll => {
                    // for device_address in self.discovered_devices.values() {
                    //     let mut device_stream = TcpStream::connect(device_address).unwrap();
                    //     device_stream.write_all("ping".as_bytes()).unwrap();
                    // }
                }
            };
        }
    }

    /// Returns the [`EventEmitter`] that can be used to send events to application's event loop.
    pub fn event_emitter(&self) -> EventEmitter {
        EventEmitter(self.event_channel.sender.clone())
    }

    /// Starts a TCP server for receiving data.
    pub fn start_data_receiver(&self) -> io::Result<()> {
        let builder = ThreadBuilder::new().name(String::from("data receiver"));
        let listener = TcpListener::bind(self.device_address)?;
        logln!("Receiving data on {}", listener.local_addr()?);

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
                    logln!("Received `{data}` from {peer_address}");
                }
            }
        })?;
        Ok(())
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

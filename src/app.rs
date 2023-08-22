use std::env;
use std::io;
use std::path::PathBuf;
use std::thread::{self, Builder as ThreadBuilder};
use std::time::Duration;

use crate::api::{Api, Command, Message};
use crate::discovery::PeerDiscoverer;
use crate::elogln;
use crate::ipc::IPCServer;
use crate::protocol::{self, PeerAddr, PeerID};
use crate::transfer::{receiver, sender};

pub struct App {
    my_id: PeerID,
    my_addr: PeerAddr,
    peer_discoverer: PeerDiscoverer,
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
        let my_id = protocol::get_my_id();
        let my_addr = protocol::get_my_addr();

        App {
            my_id,
            my_addr,
            peer_discoverer: PeerDiscoverer::new(my_id, my_addr),
            save_location: PathBuf::from(home_path),
        }
    }

    /// Starts the main event loop.
    ///
    /// **NOTE:** This function always blocks the current thread.
    pub fn run(&mut self) -> io::Result<()> {
        self.spawn_file_receiver()?;
        self.peer_discoverer.start()?;
        self.peer_discoverer.announce_peer()?;

        // Wait for a short duration to allow other threads to fully start up.
        thread::sleep(Duration::from_millis(20));
        let api = Api::new(IPCServer::new()?);

        for message in api.incoming_messages() {
            if let Err(e) = self.handle_api_message(message) {
                elogln!("Failed to handle an ipc message: {e}");
            };
        }
        Ok(())
    }

    fn spawn_file_receiver(&self) -> io::Result<()> {
        let receiving_addr = self.my_addr;
        let save_location = self.save_location.clone();
        let builder = ThreadBuilder::new().name(String::from("file_receiver"));
        builder.spawn(move || receiver::receive_files(receiving_addr, save_location))?;
        Ok(())
    }

    fn handle_api_message(&self, mut msg: Message) -> io::Result<()> {
        match msg.command() {
            Command::MyID => msg.response(self.my_id),
            Command::MyAddr => msg.response(self.my_addr),
            Command::Peers => match self.peer_discoverer.get_discovered_peer_ids() {
                Some(ids) => {
                    let ids = ids.iter().map(|&id| format!("{id}\n")).collect::<String>();
                    msg.response(ids)
                }
                None => msg.response("No peers found"),
            },
            Command::Send(file_path) => match self.peer_discoverer.get_discovered_peer_addrs() {
                Some(addrs) => sender::send_file_to_all(&addrs, file_path)
                    .or_else(|_| msg.response("Failed to send file")),
                None => msg.response("No peers found"),
            },
            Command::SendTo(peer_id, file_path) => {
                match self.peer_discoverer.find_peer_addr_by_id(*peer_id) {
                    Some(addr) => sender::send_file_to(addr, file_path)
                        .or_else(|_| msg.response("Failed to send file: {e}")),
                    None => msg.response("No peers found that matches the given identifier"),
                }
            }
        }
    }
}

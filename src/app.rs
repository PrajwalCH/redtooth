use std::io::{self, Error};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs, thread};

use crate::api::{Api, Message, Request};
use crate::discovery::PeerDiscovery;
use crate::elogln;
use crate::ipc::IPCServer;
use crate::protocol::{self, PeerAddr, PeerID};
use crate::transfer::{receiver, sender};

#[cfg(not(windows))]
const HOME_ENV_KEY: &str = "HOME";
#[cfg(windows)]
const HOME_ENV_KEY: &str = "USERPROFILE";
/// Directory where all the received files will live.
const DIR_NAME: &str = env!("CARGO_PKG_NAME");

pub struct App {
    my_id: PeerID,
    my_addr: PeerAddr,
    peer_discovery: PeerDiscovery,
    config: Config,
}

impl App {
    /// Creates a new instance of `App` with all the necessary setup.
    pub fn new() -> App {
        let my_id = protocol::get_my_id();
        let my_addr = protocol::get_my_addr();

        App {
            my_id,
            my_addr,
            peer_discovery: PeerDiscovery::new(my_id, my_addr),
            config: Config::default(),
        }
    }

    /// Starts the main event loop.
    ///
    /// **NOTE:** This function always blocks the current thread.
    pub fn run(&mut self) -> io::Result<()> {
        let save_location_exists = self.config.save_location.try_exists().map_err(|err| {
            Error::new(err.kind(), "failed to check the existence of save location")
        })?;

        if !save_location_exists {
            fs::create_dir(&self.config.save_location)?;
        }
        self.spawn_file_receiver()?;
        self.peer_discovery.spawn()?;
        self.peer_discovery.announce_peer()?;

        // Wait for a short duration to allow other threads to fully start up.
        thread::sleep(Duration::from_millis(20));
        let api = Api::new(IPCServer::new()?);

        for request in api.incoming_requests() {
            if let Err(e) = self.handle_api_request(request) {
                elogln!("Failed to handle an api request: {e}");
            };
        }
        Ok(())
    }

    fn spawn_file_receiver(&self) -> io::Result<()> {
        let receiving_addr = self.my_addr;
        let save_location = self.config.save_location.clone();

        thread::Builder::new()
            .name(String::from("file_receiver"))
            .spawn(move || receiver::receive_files(receiving_addr, save_location))?;

        Ok(())
    }

    fn handle_api_request(&self, mut req: Request) -> io::Result<()> {
        match req.message() {
            Message::MyID => req.response(self.my_id),
            Message::MyAddr => req.response(self.my_addr),
            Message::Peers => match self.peer_discovery.get_discovered_peer_ids() {
                Some(ids) => {
                    let ids = ids.iter().map(|&id| format!("{id}\n")).collect::<String>();
                    req.response(ids)
                }
                None => req.response("No peers found"),
            },
            Message::Send(file_path) => match self.peer_discovery.get_discovered_peer_addrs() {
                Some(addrs) => sender::send_file_to_all(&addrs, file_path)
                    .or_else(|_| req.response("Failed to send file")),
                None => req.response("No peers found"),
            },
            Message::SendTo(peer_id, file_path) => {
                match self.peer_discovery.find_peer_addr_by_id(*peer_id) {
                    Some(addr) => sender::send_file_to(addr, file_path)
                        .or_else(|_| req.response("Failed to send file: {e}")),
                    None => req.response("No peers found that matches the given identifier"),
                }
            }
        }
    }
}

struct Config {
    /// Path where the received file will be saved.
    save_location: PathBuf,
}

impl Default for Config {
    fn default() -> Config {
        let home = env::var(HOME_ENV_KEY).expect("your OS should set env variable {HOME_ENV_KEY}");

        Config {
            save_location: Path::new(&home).join(DIR_NAME),
        }
    }
}

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread::{self, Builder as ThreadBuilder};
use std::time::Duration;

use crate::cli::{self, Command};
use crate::elogln;
use crate::file_transfer::{receiver, sender};
use crate::peer_discoverer::PeerDiscoverer;
use crate::protocol::{self, PeerAddr, PeerID};

#[allow(dead_code)]
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
        // TODO: Implement either ipc or http api server so that both cli and web ui can talk.
        let mut cli_input_buffer = String::new();

        loop {
            cli_input_buffer.clear();
            print!("> ");
            io::stdout().flush()?;

            match cli::read_command(&mut cli_input_buffer) {
                Ok(c) => self.handle_cli_command(c),
                Err(e) => elogln!("Failed to read input: {e}"),
            }
        }
    }

    fn spawn_file_receiver(&self) -> io::Result<()> {
        let receiving_addr = self.my_addr;
        let save_location = self.save_location.clone();
        let builder = ThreadBuilder::new().name(String::from("file_receiver"));
        builder.spawn(move || receiver::receive_files(receiving_addr, save_location))?;
        Ok(())
    }

    fn handle_cli_command(&self, cmd: Command) {
        match cmd {
            Command::MyIp => {
                println!("{}", self.my_addr.ip());
            }
            Command::List => {
                if let Some(ids) = self.peer_discoverer.get_discovered_peer_ids() {
                    for id in ids {
                        println!("{id}");
                    }
                    return;
                }
                println!("No peers found");
            }
            Command::Send(file_path) => {
                let Some(addrs) = self.peer_discoverer.get_discovered_peer_addrs() else {
                    println!("No peers found");
                    return;
                };
                if let Err(e) = sender::send_file_to_all(&addrs, file_path) {
                    eprintln!("Failed to send file: {e}");
                }
            }
            Command::SendTo(peer_id, file_path) => {
                let Some(addr) = self.peer_discoverer.find_peer_addr_by_id(peer_id) else {
                    println!("No peers found that matches the given identifier");
                    return;
                };
                if let Err(e) = sender::send_file_to(addr, file_path) {
                    eprintln!("Failed to send file: {e}");
                }
            }
            Command::Unknown => println!("Unknown command"),
        }
    }
}

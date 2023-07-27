mod app;
mod cli;
mod discovery_server;
mod interface;
mod macros;
mod protocol;
mod receiver;
mod sender;

use crate::app::App;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.run()
}

mod app;
mod cli;
mod interface;
mod macros;
mod peer_discoverer;
mod protocol;
mod receiver;
mod sender;

use crate::app::App;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.run()
}

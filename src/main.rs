mod api;
mod app;
mod cli;
mod config;
mod discovery;
mod interface;
mod ipc;
mod macros;
mod protocol;
mod transfer;

use crate::app::App;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.run()
}

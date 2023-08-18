mod app;
mod cli;
mod file_transfer;
mod interface;
mod ipc;
mod macros;
mod discovery;
mod protocol;

use crate::app::App;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.run()
}

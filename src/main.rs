mod app;
mod discovery_server;
mod interface;
mod macros;
mod protocol;
mod sender;

use crate::app::App;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.run()
}

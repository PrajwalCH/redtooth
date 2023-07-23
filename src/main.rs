mod app;
mod device;
mod discovery_server;
mod interface;
mod macros;
mod sender;

use crate::app::App;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.run()
}

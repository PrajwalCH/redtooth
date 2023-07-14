mod app;
mod device;
mod discovery_server;
mod interface;

use crate::app::{App, Event};

fn main() -> std::io::Result<()> {
    let app = App::new();
    let event = app.event_emitter();
    app.run()?;

    loop {
        event.emit(Event::PingAll).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

mod app;
mod device;
mod discovery_server;
mod interface;
mod macros;

use std::thread;

use crate::app::{App, Event};

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    let event_emitter = app.event_emitter();

    let event_loop_thread = thread::Builder::new()
        .name(String::from("event loop"))
        .spawn(move || app.run())?;

    loop {
        if event_loop_thread.is_finished() {
            event_loop_thread.join().unwrap()?;
            break;
        }
        event_emitter.emit(Event::PingAll);
        thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(())
}

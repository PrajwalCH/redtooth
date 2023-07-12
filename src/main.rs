mod device;
mod discovery_server;
mod interface;

use crate::device::Device;

fn main() -> std::io::Result<()> {
    let mut current_device = Device::new();
    current_device.run()
}

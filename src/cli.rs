use std::io::{self, BufRead};
use std::net::Ipv4Addr;
use std::str::FromStr;

pub enum Command {
    /// Not a known command.
    Unknown,
    /// Show the IP address of the device.
    MyIp,
    /// Display all the discovered devices address.
    List,
    /// Send a file to all the addresses of the devices.
    Send(String),
    /// Send a file to the given address of a device.
    SendTo(String, Ipv4Addr),
}

pub fn read_command(input_buffer: &mut String) -> io::Result<Command> {
    let mut stdin = io::stdin().lock();
    stdin.read_line(input_buffer)?;

    let mut it = input_buffer.split(' ');
    let command = it.next().unwrap_or_default().trim();
    let command = match command {
        "myip" => Command::MyIp,
        "list" => Command::List,
        "send" => Command::Send(it.next().unwrap().trim().to_string()),
        "sendto" => {
            let file_path = it.next().unwrap().trim().to_string();
            let device_addr = Ipv4Addr::from_str(it.next().unwrap().trim()).unwrap();
            Command::SendTo(file_path, device_addr)
        }
        _ => Command::Unknown,
    };
    Ok(command)
}

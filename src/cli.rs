use std::io;
use std::io::BufRead;

use crate::protocol::DeviceID;

pub enum Command<'buf> {
    /// Unknown or unrecognized command
    Unknown,
    /// Display the IP address of the current device.
    MyIp,
    /// Display the identifiers of all the discovered devices.
    List,
    /// Send a file to all the devices.
    Send(&'buf str),
    /// Send a file to the device that matches the given identifier.
    SendTo(DeviceID, &'buf str),
}

pub fn read_command(input_buffer: &mut String) -> io::Result<Command> {
    let mut stdin = io::stdin().lock();
    stdin.read_line(input_buffer)?;

    let mut it = input_buffer.split(' ');
    let command = it.next().unwrap_or_default().trim();
    let command = match command {
        "myip" => Command::MyIp,
        "list" => Command::List,
        "send" => Command::Send(it.next().unwrap().trim()),
        "sendto" => {
            let device_id = it.next().unwrap().trim().parse::<DeviceID>().unwrap();
            let file_path = it.next().unwrap().trim();
            Command::SendTo(device_id, file_path)
        }
        _ => Command::Unknown,
    };
    Ok(command)
}

use std::fmt;
use std::io::{self, Read};
use std::net::{TcpListener, TcpStream};

use crate::app::{Event, EventEmitter};
use crate::protocol::{DeviceAddress, FilePacket, FilePacketFromBytesError};
use crate::{elogln, logln};

pub fn start_file_receiving(addr: DeviceAddress, event_emitter: EventEmitter) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    logln!("Receiving data on {addr}");

    for stream in listener.incoming().flatten() {
        match read_data_from_peer(stream) {
            Ok(file_packet) => event_emitter.emit(Event::DataReceived(file_packet)),
            Err(e) => elogln!("{e}"),
        }
    }
    Ok(())
}

enum DataReadError {
    DataParseError(FilePacketFromBytesError),
    IoError(io::Error),
}

impl fmt::Display for DataReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::DataReadError::*;

        match self {
            DataParseError(e) => {
                write!(f, "{e}")
            }
            IoError(e) => write!(f, "Unable to read data properly: {e}"),
        }
    }
}

fn read_data_from_peer(mut stream: TcpStream) -> Result<FilePacket, DataReadError> {
    let mut data: Vec<u8> = Vec::new();
    let data_len = stream
        .read_to_end(&mut data)
        .map_err(DataReadError::IoError)?;
    logln!("Received data of {data_len} bytes");

    let data = FilePacket::from_bytes(&data).map_err(DataReadError::DataParseError)?;
    Ok(data)
}

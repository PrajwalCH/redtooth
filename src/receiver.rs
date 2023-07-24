use std::fmt;
use std::io::{self, Read};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;

use crate::app::{Event, EventEmitter};
use crate::protocol::{DataHeader, DataHeaderParseError};
use crate::protocol::{DeviceAddress, DATA_SECTIONS_SEPARATOR};
use crate::{elogln, logln};

pub fn start_file_receiving(addr: DeviceAddress, event_emitter: EventEmitter) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    logln!("Receiving data on {addr}");

    for stream in listener.incoming().flatten() {
        match read_data_from_peer(stream) {
            Ok((header, contents)) => {
                event_emitter.emit(Event::DataReceived(header, contents));
            }
            Err(e) => {
                elogln!("{e}");
                continue;
            }
        }
    }
    Ok(())
}

enum DataReadError {
    SectionsSeparatorNotFound,
    DataHeaderParseError(DataHeaderParseError),
    IoError(io::Error),
}

impl fmt::Display for DataReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::DataReadError::*;

        match self {
            SectionsSeparatorNotFound => {
                write!(f, "Data sections separator is missing from the data")
            }
            DataHeaderParseError(e) => {
                write!(f, "Unable to parse the header of data: {e}")
            }
            IoError(e) => write!(f, "Unable to read data properly: {e}"),
        }
    }
}

fn read_data_from_peer(mut stream: TcpStream) -> Result<(DataHeader, Vec<u8>), DataReadError> {
    // Data should be in the following format:
    // ```
    // file_name: filename.jpeg
    // ::
    // file contents
    // ```
    let mut data: Vec<u8> = Vec::new();
    let data_len = stream
        .read_to_end(&mut data)
        .map_err(DataReadError::IoError)?;

    logln!("Received data of {data_len} bytes");

    let separator_len = DATA_SECTIONS_SEPARATOR.len();
    let separator_index = data
        .windows(separator_len)
        .position(|bytes| bytes == DATA_SECTIONS_SEPARATOR)
        .ok_or(DataReadError::SectionsSeparatorNotFound)?;

    let header = std::str::from_utf8(&data[..separator_index]).unwrap_or_default();
    let header = DataHeader::from_str(header).map_err(DataReadError::DataHeaderParseError)?;
    // Skip all the separator bytes.
    let file_contents = data.get(separator_index + separator_len..);
    // If a valid header and separator are present but the contents are missing,
    // declare it as a empty.
    let file_contents = file_contents.unwrap_or_default().to_owned();
    Ok((header, file_contents))
}


/**
# Protocol

## PING (client -> server)
Asks server if he's active
### Data send
`[1 byte flag PING]`

## PONG (server -> client)
Response to a ping req
### Data send
`[1 byte flag PONG]`

## ACK_REQ (client -> server)
Asks if server wants to receive file(s)
Transmitted as list of `[4 byte file-id][8 byte file-size][2 byte name_len][name utf8]`
## Data send
`[1 byte flag ACK_REQ][4 byte length of list][string (list)]`
*/
pub mod flags {
    pub const PING: u8 = 0x10;
    pub const PONG: u8 = 0x20;
    pub const ACK_REQ : u8 = 0x11;
    pub const ACK_RES : u8 = 0x12;
}


#[derive(Debug)]
pub struct FileMeta {
    pub size: u64,
    pub path: PathBuf
}

impl FileMeta {
    pub fn from(path: &str) -> io::Result<FileMeta> {
        let meta = std::fs::metadata(path)?;

        Ok(FileMeta {
            size: meta.len(),
            path: PathBuf::from(path)
        })
    }
}

#[derive(Debug)]
pub enum Parsed {
    Ping,
    Pong,
    AckReq(Vec<FileMeta>),
    AckRes(bool)
}

impl Parsed {
    pub fn to_buf(&self) -> Box<[u8]> {
        match self {
            Parsed::Ping => Box::new([flags::PING]),
            Parsed::Pong => Box::new([flags::PONG]),
            _ => unimplemented!()
        }
    }
}

use std::io::{self, Error, ErrorKind, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

pub  fn parse(stream: &mut TcpStream) -> io::Result<Parsed> {
    let mut reader : BufReader<& TcpStream> = BufReader::new(stream);

    let packet_type : u8 = {
        let mut d = [0u8];
        reader.read_exact(&mut d)?;
        d[0]
    };

    #[cfg(debug_assertions)]
    println!("packet_type: {:X}", packet_type);

    match packet_type {
        flags::PING => return Ok(Parsed::Ping),
        flags::PONG => return Ok(Parsed::Pong),
        flags::ACK_REQ => {
            let mut r_dat = [0u8, 0u8];
            reader.read_exact(&mut r_dat)?;
        }
        _ => {}
    }

    Err(Error::new(ErrorKind::InvalidData, "Can't parse stream"))
}

pub  fn send_slice<W : Write>(stream: &mut W, data: &[u8]) -> io::Result<()> {
    #[cfg(debug_assertions)]
    println!("sending {} bytes", data.len());
    stream.write_all(data)
}
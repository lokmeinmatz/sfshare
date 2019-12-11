
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
Transmitted as text
## Data send
`[1 byte flag ACK_REQ][2 byte length of string][string (length as described)]`
*/
pub mod flags {
    pub const PING: u8 = 0x10;
    pub const PONG: u8 = 0x20;
    pub const ACK_REQ : u8 = 0x11;
    pub const ACK_RES : u8 = 0x12;
}



#[derive(Debug)]
pub enum Parsed {
    Ping,
    Pong,
    AckReq(String),
    AckRes(bool)
}

use std::io::{self, Error, ErrorKind, BufReader, Read, Write};
use std::net::TcpStream;

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

pub  fn send_slice(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    #[cfg(debug_assertions)]
    println!("sending Packet to {}", stream.peer_addr().map(|e| format!("{}", e)).unwrap_or_else(|e| e.to_string()));
    stream.write_all(data)
}
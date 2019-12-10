pub const FLAG_PING: u8 = 0x2f;
pub const FLAG_PONG: u8 = 0x1f;


#[derive(Debug)]
pub enum Parsed {
    Ping,
    Pong
}

use crate::AsyncResult;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::io::{BufReader};
use std::io::{Error, ErrorKind};

pub async fn parse(stream: &mut TcpStream) -> AsyncResult<Parsed> {
    let mut reader : BufReader<& TcpStream> = BufReader::new(stream);

    let packet_type : u8 = {
        let mut d = [0u8];
        reader.read_exact(&mut d).await?;
        d[0]
    };

    #[cfg(debug_assertions)]
    println!("packet_type: {:X}", packet_type);

    match packet_type {
        FLAG_PING => return Ok(Parsed::Ping),
        FLAG_PONG => return Ok(Parsed::Pong),
        _ => {}
    }

    Err(Box::new(Error::new(ErrorKind::InvalidData, "Can't parse stream")))
}

pub async fn send_slice(stream: &mut TcpStream, data: &[u8]) -> AsyncResult<()> {
    #[cfg(debug_assertions)]
    println!("sending Packet to {}", stream.peer_addr().map(|e| format!("{}", e)).unwrap_or_else(|e| e.to_string()));
    stream.write_all(data).await.map_err(Box::from)
}
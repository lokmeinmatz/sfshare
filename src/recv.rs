use std::{
    net::{TcpListener, ToSocketAddrs, IpAddr},
    io
};
use crate::transport;
use crossterm::style::Colorize;
use std::net::Ipv6Addr;

fn tcp_handler() -> io::Result<()> {

    #[cfg(debug_assertions)]
    println!("tcp_handler()");

    let listener = TcpListener::bind((IpAddr::V6(Ipv6Addr::LOCALHOST), 5123u16))?; // 2
    let mut incoming = listener.incoming();


    // get adresses to connect to
    println!("{}", "Ip Adresses to connect to".black().on_green());
    for adapter in ipconfig::get_adapters().map_err(|e|io::Error::from(io::ErrorKind::NotConnected))? {
        println!("{:30} | {}", adapter.adapter_name(), adapter.description());

        for ip in adapter.ip_addresses() {
            match ip {
                //IpAddr::V4(addr) => println!(" IPv4 > {}:5123", addr),
                IpAddr::V6(addr) => println!(" IPv6 > [{}]:5123", addr),
                _ => {}
            }
            
        }
    }

    println!("Waiting for files...");

    while let Some(stream) = incoming.next() { // 3
        let mut stream = stream?;

        let parsed = match transport::parse(&mut stream) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Unknown packet: {}", e);
                continue
            }
        };
        #[cfg(debug_assertions)]
        println!("Packet: {:?}", parsed);

        match parsed {
            transport::Parsed::Ping => {
                // send pong back
                transport::send_slice(&mut stream, &[transport::flags::PONG])?;
            },
            transport::Parsed::Pong => {
                println!("Received pong... why?!");
            },
            transport::Parsed::AckReq(req) =>
        }

    }

    Ok(())
}

pub fn recv() -> io::Result<()> {
    //  tasks needed:
    //  - tcp-listener : handles ping and receiving of files
    //  - terminal-handler: ask for confirmation of receiving and handle settings
    // communicate via channels?
    tcp_handler()
}
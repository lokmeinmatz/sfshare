use std::{
    net::{TcpListener, IpAddr},
    io
};
use crate::transport;
use crossterm::style::Colorize;
use std::net::Ipv6Addr;
use std::intrinsics::transmute;

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

    'new_con: while let Some(stream) = incoming.next() { // 3
        let mut stream = stream?;

        let parsed = match transport::parse(&mut stream) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Unknown packet: {}", e);
                continue 'new_con;
            }
        };
        #[cfg(debug_assertions)]
        println!("Packet: {:?}", parsed);

        match parsed {
            transport::Parsed::Ping => {
                // send pong back
                transport::send_slice(&mut stream, transport::Parsed::Pong.to_buf().as_ref())?;
            },
            transport::Parsed::Pong => {
                println!("Received pong... why?!");
            },
            transport::Parsed::AckReq(req) => {
                // ask if we ant to receive this
                let file_size_sum = req.iter().fold(0, |acc, e| e.size + acc);

                println!("{}", "New Transmission Request".yellow().on_dark_magenta());

                println!("\nDo you want to receive {} file{} with a total size of {}mb", req.len(), if req.len() > 1 {"s"} else {""}, file_size_sum as f64 / 1_000_000f64);

                let mut l = String::new();
                while l.trim() != "y" && l.trim() != "yes" {
                    println!("[y, yes] / [n, no]")
                    io::stdin().read_line(&mut l);
                    if l.trim() == "n" || l.trim() == "no" {
                        println!("You denied the request. Listening for new requests.");
                        transport::send_slice(&mut stream, transport::Parsed::AckRes(false).to_buf().as_ref())?;
                        continue 'new_con;
                    }
                }



            },
            _ => unimplemented!()
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
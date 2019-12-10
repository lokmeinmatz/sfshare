use async_std::{
    prelude::*, // 1
    task, // 2
    net::{TcpListener, ToSocketAddrs, IpAddr},
    sync::{channel, Sender, Receiver}
};
use crate::AsyncResult;
use crate::transport;
use crossterm::style::Colorize;

async fn tcp_handler() -> AsyncResult<()> {

    #[cfg(debug_assertions)]
    println!("tcp_handler()");

    let listener = TcpListener::bind(("::1".parse::<IpAddr>()?, 5123u16)).await?; // 2
    let mut incoming = listener.incoming();


    // get adresses to connect to
    println!("{}", "Ip Adresses to connect to".black().on_green());
    for adapter in ipconfig::get_adapters()? {
        println!("{:30} | {}", adapter.adapter_name(), adapter.description());

        for ip in adapter.ip_addresses() {
            match ip {
                IpAddr::V4(addr) => println!(" IPv4 > {}:5123", addr),
                IpAddr::V6(addr) => println!(" IPv6 > [{}]:5123", addr),
            }
            
        }
    }

    println!("Waiting for files...");

    while let Some(stream) = incoming.next().await { // 3
        let mut stream = stream?;

        let parsed = match transport::parse(&mut stream).await {
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
                transport::send_slice(&mut stream, &[transport::FLAG_PONG]).await?;
            },
            transport::Parsed::Pong => {
                println!("Received pong... why?!");
            }
        }

    }

    Ok(())
}

pub fn recv() -> AsyncResult<()> {
    // async tasks needed:
    //  - tcp-listener : handles ping and receiving of files
    //  - terminal-handler: ask for confirmation of receiving and handle settings
    // communicate via channels?
    task::block_on(tcp_handler())?;

    Ok(())
}
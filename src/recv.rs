use crate::transport;
use crossterm::style::Colorize;
use std::net::Ipv6Addr;
use std::{
    io,
    net::{IpAddr, TcpListener},
};
use crate::transport::{Parsed, FileMeta};
use std::io::{BufWriter, BufReader};
use std::fs::File;
use std::collections::{HashSet, HashMap};
use std::iter::FromIterator;

fn tcp_handler() -> io::Result<()> {
    #[cfg(debug_assertions)]
    println!("tcp_handler()");

    let listener = TcpListener::bind((IpAddr::V6(Ipv6Addr::LOCALHOST), 5123u16))?; // 2
    let mut incoming = listener.incoming();

    // get adresses to connect to
    println!("{}", "Ip Adresses to connect to".black().on_green());
    for adapter in
        ipconfig::get_adapters().map_err(|_| io::Error::from(io::ErrorKind::NotConnected))?
    {
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

    'new_con: while let Some(stream) = incoming.next() {
        // 3
        let mut stream = stream?;

        let mut reader = BufReader::new(stream.try_clone()?);

        println!("new connection");

        'new_packet: loop {

            println!("waiting for next packet");

            let parsed = match transport::parse(&mut reader) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Unknown packet: {}", e);
                    continue 'new_packet;
                }
            };
            #[cfg(debug_assertions)]
            println!("Packet: {:?}", parsed);

            match parsed {
                transport::Parsed::Ping => {
                    // send pong back
                    transport::send_slice(&mut stream, transport::Parsed::Pong.to_buf().as_ref())?;
                }
                transport::Parsed::Pong => {
                    println!("Received pong... why?!");
                }
                transport::Parsed::AckReq(mut req) => {
                    // ask if we ant to receive this
                    let file_size_sum = req.iter().fold(0, |acc, e| e.size + acc);

                    println!("{}", "New Transmission Request".yellow().on_dark_magenta());

                    println!(
                        "\nDo you want to receive {} file{} with a total size of {}mb",
                        req.len(),
                        if req.len() > 1 { "s" } else { "" },
                        file_size_sum as f64 / 1_000_000f64
                    );

                    let mut l = String::new();
                    while l.trim() != "y" && l.trim() != "yes" {
                        println!("[y, yes] / [n, no]");
                        io::stdin().read_line(&mut l)?;
                        if l.trim() == "n" || l.trim() == "no" {
                            println!("You denied the request. Listening for new requests.");
                            transport::send_slice(
                                &mut stream,
                                transport::Parsed::AckRes(false).to_buf().as_ref(),
                            )?;
                            continue 'new_con;
                        }
                    }

                    transport::send_slice(&mut stream, transport::Parsed::AckRes(true).to_buf().as_ref())?;

                    let mut files_waiting: HashMap<u32, FileMeta> = HashMap::from_iter(req.drain(..).map(|e| (e.id, e)));

                    // we need to store the current open file meta data
                    let mut current_file_meta: Option<FileMeta> = None;
                    let mut current_file_writer: Option<BufWriter<File>> = None;
                    let mut current_file_checksum = 0u64;

                    loop {
                        match transport::parse(&mut reader)? {
                            Parsed::FileBlock { id, data } => {
                                current_file_checksum = (current_file_checksum + data.iter().fold(0u64, |acc, b| acc + *b as u64 )) % transport::CHECKSUM_MOD;
                                match (current_file_meta, current_file_writer) {
                                    (Some(meta), Some(writer)) => {

                                    },
                                    _ => {
                                        // create new file if want to receive
                                        if let Some(fm) = files_waiting.remove(&id) {
                                            
                                            current_file_meta = Some(fm);


                                        }
                                        else {
                                            eprintln!("Didn't await this file :/");
                                            // TODO handle wrong file id
                                        }
                                    }
                                }
                            },
                            Parsed::FileEnd(cs) => {
                                if cs != current_file_checksum {
                                    eprintln!("Checksum not identical! calculated: {} | received: {}", current_file_checksum, cs);
                                }
                                else {
                                    println!("File transmission success");
                                }
                                break;
                            }
                            e => {
                                eprintln!("Received wrong packet :( : {:?}", e);
                                continue 'new_con
                            }
                        }
                    }
                }
                _ => unimplemented!(),
            }
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

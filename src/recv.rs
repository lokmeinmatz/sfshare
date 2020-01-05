use crate::transport;
use crate::transport::{FileMeta, Parsed};

use crossterm::style::{Colorize, Print};
use crossterm::cursor::{MoveDown, MoveUp};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{queue};

use std::collections::{HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write, stdout};
use std::iter::FromIterator;
use std::net::Ipv6Addr;
use std::path::PathBuf;
use std::{
    io,
    net::{IpAddr, TcpListener},
};

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
                    match e.kind() {
                        io::ErrorKind::UnexpectedEof => {
                            // connection is closed
                            println!("Connection closed");
                            continue 'new_con;
                        }
                        io::ErrorKind::InvalidData => {
                            eprintln!("Unknown packet / invalid data: {}", e);
                            continue 'new_packet;
                        }
                        _ => {
                            eprintln!("Unknown error {} : try again or contact developer!", e);
                            return Err(e);
                        }
                    }
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
                    let files_total = req.len();

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

                    transport::send_slice(
                        &mut stream,
                        transport::Parsed::AckRes(true).to_buf().as_ref(),
                    )?;

                    let mut files_waiting: HashMap<u32, FileMeta> =
                        HashMap::from_iter(req.drain(..).map(|e| (e.id, e)));

                    // we need to store the current open file meta data
                    let mut current_file_meta: Option<FileMeta> = None;
                    let mut current_file_writer: Option<BufWriter<File>> = None;
                    let mut current_file_checksum = 0u64;

                    queue!(stdout(), MoveDown(3)).unwrap();
                    let mut blocks_till_redraw = 0;
                    let mut bytes_recvd: u64 = 0;
                    let mut files_received = 0;
                    loop {
                        match transport::parse(&mut reader)? {
                            Parsed::FileBlock { id, data } => {
                                bytes_recvd += data.len() as u64;

                                if blocks_till_redraw <= 0 || data.is_empty() {

                                    let percent_send = bytes_recvd as f64 / file_size_sum as f64;

                                    queue!(
                                            stdout(),
                                            MoveUp(3),
                                            Clear(ClearType::CurrentLine),
                                            Print(format!(
                                                "Copying file with id {} | {}/{}\n",
                                                id, files_received, files_total
                                            )),
                                            Clear(ClearType::CurrentLine),
                                            Print(format!(
                                                "{:.3}mb of {:.3}mb send ({:.2}%)\n",
                                                (bytes_recvd as f64 / 1_000_000.0),
                                                (file_size_sum as f64 / 1_000_000.0),
                                                percent_send * 100.0
                                            )),
                                            Clear(ClearType::CurrentLine),
                                            Print(format!(
                                                "[{}>{}]\n",
                                                "=".repeat((percent_send * 20.0).floor() as usize),
                                                " ".repeat(((1.0 - percent_send) * 20.0).ceil() as usize)
                                            ))
                                        ).expect("Failed to display loading bar");
                                    blocks_till_redraw = 20.min(file_size_sum as i64 / (data.len() as i64 * 10 + 1));
                                }
                                blocks_till_redraw -= 1;
                                current_file_checksum = (current_file_checksum
                                    + data.iter().fold(0u64, |acc, b| acc + *b as u64))
                                    % transport::CHECKSUM_MOD;
                                match (&mut current_file_meta, &mut current_file_writer) {

                                    (Some(meta), Some(writer)) => {



                                        if meta.id != id {
                                            return Err(io::Error::new(
                                                io::ErrorKind::PermissionDenied,
                                                "Wrong file id: not accepted",
                                            ));
                                        }

                                        writer.write_all(&data)?;
                                    }
                                    _ => {

                                        // create new file if want to receive
                                        if let Some(mut fm) = files_waiting.remove(&id) {
                                            assert_eq!(fm.path, None);
                                            files_received += 1;
                                            let pbuf = PathBuf::from(format!("./{}", fm.name));

                                            let mut bwriter = BufWriter::new(File::create(&pbuf)?);

                                            bwriter.write_all(&data)?;

                                            fm.path = Some(pbuf);
                                            current_file_meta = Some(fm);
                                            current_file_writer = Some(bwriter);
                                        } else {
                                            eprintln!("Didn't await this file :/");
                                            // TODO handle wrong file id
                                        }
                                    }
                                }
                            }
                            Parsed::FileEnd(cs) => {
                                if cs != current_file_checksum {
                                    eprintln!(
                                        "Checksum not identical! calculated: {} | received: {}",
                                        current_file_checksum, cs
                                    );
                                } else {
                                    println!("File transmission success! Checksum identical");
                                    current_file_meta = None;
                                    current_file_writer = None;
                                    current_file_checksum = 0;
                                }

                                if files_waiting.is_empty() {
                                    println!("All files received!");
                                    continue 'new_con;
                                }
                            }
                            e => {
                                eprintln!("Received wrong packet :( : {:?}", e);
                                continue 'new_con;
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

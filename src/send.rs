use crate::AppState;

pub enum SendFiles {
    Single(String),
    Selected(Vec<String>),
    AllNonRecursive,
    AllRecursive
}

impl std::fmt::Display for SendFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result<> {
        match self {
            SendFiles::Single(name) => f.write_str(name),
            SendFiles::Selected(names) => write!(f, "{:?}", names),
            SendFiles::AllNonRecursive => f.write_str("All in this folder (not recursive)"),
            SendFiles::AllRecursive => f.write_str("All in this folder and subfolders"),
        }
    }
}

use std::io;
use std::net::{TcpStream, SocketAddr};
use crate::transport::FileMeta;

fn get_file_meta(files: &SendFiles) -> Vec<FileMeta> {

    match files {
        SendFiles::Single(n) => FileMeta::from(n).and_then(|m| Ok(vec![m])).unwrap_or_else(|_| vec![]),
        // TODO handle file not found here
        SendFiles::Selected(selection) => selection.iter().map(|n| FileMeta::from(n)).filter_map(|m| m.ok()).collect(),
        _ => unimplemented!()
    }
}


use crate::transport;
use std::io::Read;

pub  fn send(state: &crate::AppState) -> io::Result<()> {
    #[cfg(debug_assertions)]
    println!("send::send()");

    match state {
        AppState::Send{to, files} => {

            // check if <to> is active (ping)
            let mut stream : TcpStream = TcpStream::connect(to)?;
            transport::send_slice(&mut stream, &[transport::flags::PING])?;
            
            // wait for ping
            {
                let mut answ = [0u8];
                stream.read_exact(&mut answ)?;

                if answ[0] != transport::flags::PONG {
                    eprintln!("Receiver can't be reached. Make sure that both are connected to the same network and sfshare is running in recv mode.");
                    return Err(io::Error::new(io::ErrorKind::NotConnected, "Not reachable"));
                }
            }

            // calculate file size
            let file_meta = get_file_meta(files);
            if file_meta.is_empty() {
                eprintln!("No files found.");
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }

            let total_size = file_meta.iter().fold(0, |acc, m| acc + m.size);

            // 1 mb or too many files
            // TODO save different limit
            if total_size > 1_000_000 || file_meta.len() > 5 {
                println!("Are you sure you want to send {} files with {}mb size total?", file_meta.len(), total_size as f64 / 1_000_000f64);

                let mut res = String::new();
                while !(res.trim() == "y" || res.trim() == "yes") {
                    println!("[y] yes / [n] no");
                    io::stdin().read_line(&mut res)?;
                    if res.trim() == "n" || res.trim() == "no" {
                        return Err(io::Error::from(io::ErrorKind::ConnectionAborted));
                    }
                }
            } 

            // continue - establish connection
            transport::send_slice(&mut stream, transport::Parsed::AckReq(vec![]).to_buf().as_ref())?;
            println!("Asked receiver if he wants to receive files...\nWaiting for answer");
            match transport::parse(&mut stream).unwrap() {
                transport::Parsed::AckRes(ack) => {
                    if !ack {
                        eprintln!("The receiver didn't accept your request :( maybe next time");
                        return Err(io::Error::from(io::ErrorKind::ConnectionRefused));
                    }
                },
                _ => {
                    eprintln!("Expected AckRes");
                    return Err(io::Error::from(io::ErrorKind::InvalidData));
                }
            }

            // receiver accepted request


        },
        _ => return Err(std::io::Error::from(std::io::ErrorKind::Other))
    }

    Ok(())
}

pub fn match_send(send_matches: &clap::ArgMatches) -> crate::AppState {
    #[cfg(debug_assertions)]
    println!("match_send()");

    // TODO customize port

    let recv : Option<std::net::IpAddr> = if let Some(a) = send_matches.value_of("RECEIVER") {
        // TODO save shortcuts for receivers
        a.parse().ok()
    }
    else {
        None
    };

    if let Some(recv) = recv {
        AppState::Send{
            to: SocketAddr::new(recv, 5123),
            files: SendFiles::Single("test.dat".to_owned()) // TODO parse files
        }            
    }
    else {
        AppState::Unknown("Receiver unknown")
    }
}
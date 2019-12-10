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
use std::path::Path;
use async_std::io;
use async_std::net::{TcpStream, SocketAddr};


fn get_file_meta(files: &SendFiles) -> Vec<FileMeta> {

    match files {
        SendFiles::Single(n) => FileMeta::from(n).and_then(|m| Ok(vec![m])).unwrap_or_else(|_| vec![]),
        // TODO handle file not found here
        SendFiles::Selected(selection) => selection.iter().map(|n| FileMeta::from(n)).filter_map(|m| m.ok()).collect(),
        _ => unimplemented!()
    }
}

struct FileMeta<'a> {
    size: usize,
    path: &'a Path
}

impl<'a> FileMeta<'a> {
    fn from(path: &'a str) -> io::Result<FileMeta> {
        let meta = std::fs::metadata(path)?;
        
        Ok(FileMeta {
            size: meta.len() as usize,
            path: Path::new(path)
        })
    }
}
use crate::transport;
use async_std::prelude::*;

pub async fn send(state: &crate::AppState) -> crate::AsyncResult<()> {
    #[cfg(debug_assertions)]
    println!("send::send()");

    match state {
        AppState::Send{to, files} => {

            // check if <to> is active (ping)
            let mut stream : TcpStream = TcpStream::connect(to).await?;
            transport::send_slice(&mut stream, &[transport::FLAG_PING]).await?;
            
            // wait for ping
            {
                let mut answ = [0u8];
                stream.read_exact(&mut answ).await?;

                if answ[0] != transport::FLAG_PONG {
                    eprintln!("Receiver can't be reached. Make sure that both are connected to the same network and sfshare is running in recv mode.");
                    return Err(Box::new(io::Error::new(io::ErrorKind::NotConnected, "Not reachable")));
                }
            }

            // calculate file size
            let file_meta = get_file_meta(files);
            if file_meta.is_empty() {
                eprintln!("No files found.");
                return Err(Box::new(io::Error::from(io::ErrorKind::NotFound)));
            }

            let total_size = file_meta.iter().fold(0, |acc, m| acc + m.size);

            // 1 mb or too many files
            // TODO save different limit
            if total_size > 1_000_000 || file_meta.len() > 5 {
                println!("Are you sure you want to send {} files with {}mb size total?", file_meta.len(), total_size as f64 / 1_000_000f64);

                let mut res = String::new();
                while !(res.trim() == "y" || res.trim() == "yes") {
                    println!("[y] yes / [n] no");
                    io::stdin().read_line(&mut res).await?;
                    if res.trim() == "n" || res.trim() == "no" {
                        return Err(Box::new(io::Error::from(io::ErrorKind::ConnectionAborted)));
                    }
                }
            } 

            // continue - establish connection

        },
        _ => return Err(Box::new(std::io::Error::from(std::io::ErrorKind::Other)))
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
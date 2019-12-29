use crate::AppState;

pub enum SendFiles {
    Single(String),
    Selected(Vec<String>),
    AllNonRecursive,
    AllRecursive,
}

impl std::fmt::Display for SendFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendFiles::Single(name) => f.write_str(name),
            SendFiles::Selected(names) => write!(f, "{:?}", names),
            SendFiles::AllNonRecursive => f.write_str("All in this folder (not recursive)"),
            SendFiles::AllRecursive => f.write_str("All in this folder and subfolders"),
        }
    }
}

use crate::transport::{FileMeta, Parsed};
use std::io;
use std::net::{SocketAddr, TcpStream};

fn get_file_meta(files: &SendFiles) -> Vec<FileMeta> {
    match files {
        SendFiles::Single(n) => FileMeta::from(n)
            .and_then(|m| Ok(vec![m]))
            .unwrap_or_else(|_| vec![]),
        // TODO handle file not found here
        SendFiles::Selected(selection) => selection
            .iter()
            .map(|n| FileMeta::from(n))
            .filter_map(|m| m.ok())
            .collect(),
        _ => unimplemented!(),
    }
}

use crate::transport;
use std::io::{Read, BufReader, ErrorKind, BufWriter};
use std::fs::File;

pub fn send(state: &crate::AppState) -> io::Result<()> {
    #[cfg(debug_assertions)]
    println!("send::send()");

    match state {
        AppState::Send { to, files } => {
            // check if <to> is active (ping)
            let mut stream: TcpStream = TcpStream::connect(to)?;
            let mut reader = BufReader::new(stream.try_clone()?);
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

            #[cfg(debug_assertions)]
            println!("{:?}", file_meta);

            let total_size = file_meta.iter().fold(0, |acc, m| acc + m.size);

            // 1 mb or too many files
            // TODO save different limit
            if total_size > 1_000_000 || file_meta.len() > 5 {
                println!(
                    "Are you sure you want to send {} files with {}mb size total?",
                    file_meta.len(),
                    total_size as f64 / 1_000_000f64
                );

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
            transport::send_slice(
                &mut stream,
                transport::Parsed::AckReq(file_meta.clone()).to_buf().as_ref(),
            )?;
            println!("Asked receiver if he wants to receive files...\nWaiting for answer");
            match transport::parse(&mut reader).unwrap() {
                transport::Parsed::AckRes(ack) => {
                    if !ack {
                        eprintln!("The receiver didn't accept your request :( maybe next time");
                        return Err(io::Error::from(io::ErrorKind::ConnectionRefused));
                    }
                }
                _ => {
                    eprintln!("Expected AckRes");
                    return Err(io::Error::from(io::ErrorKind::InvalidData));
                }
            }

            println!("starting to send files...");
            // receiver accepted request
            for (i, fm) in file_meta.iter().enumerate() {
                send_file(fm, &mut stream, (i, file_meta.len()))?;
            }

        }
        _ => return Err(std::io::Error::from(std::io::ErrorKind::Other)),
    }

    Ok(())
}

pub fn send_file(fm: &FileMeta, stream: &mut TcpStream, k_of_n: (usize, usize)) -> io::Result<()> {

    use crossterm::cursor::{MoveDown, MoveUp};
    use crossterm::terminal::{Clear, ClearType};
    use crossterm::style::{Print};
    use crossterm::{queue, execute};
    use std::io::{stdout, Write};

    let path = if let Some(p) = &fm.path {p.clone()} else {
        return Err(io::Error::from(io::ErrorKind::NotFound));
    };
    let mut reader = BufReader::new(File::open(path)?);


    let mut bytes_send = 0u64;

    let mut checksum = 0u64;

    queue!(stdout(), MoveDown(3));
    let mut blocks_till_redraw = 0;
    loop {

        let percent_send = bytes_send as f64 / fm.size as f64;
        let block_size: u16 = (fm.size - bytes_send).min(1300) as u16;



        if blocks_till_redraw <= 0 || block_size == 0 {
            queue!(stdout(),
                MoveUp(3),
                Clear(ClearType::CurrentLine),
                Print(format!("Copying file {} | {}/{}\n", fm.name, k_of_n.0, k_of_n.1)),
                Clear(ClearType::CurrentLine),
                Print(format!("{:.3}mb of {:.3}mb send ({:.2}%)\n", (bytes_send as f64 / 1_000_000.0), (fm.size as f64 / 1_000_000.0), percent_send * 100.0)),
                Clear(ClearType::CurrentLine),
                Print(format!("[{}>{}]\n", "=".repeat((percent_send * 20.0).floor() as usize), " ".repeat(((1.0 - percent_send) * 20.0).ceil() as usize)))
            );
            blocks_till_redraw = 20.min((fm.size as i64 / (block_size as i64 * 10 + 1)));
        }

        if block_size == 0 {break;}

        let mut data = vec![0u8; block_size as usize];


        reader.read_exact(&mut data)?;

        checksum = (checksum + data.iter().fold(0u64, |acc, b| acc + *b as u64 )) % transport::CHECKSUM_MOD;

        let packet = Parsed::FileBlock {
            id: fm.id,
            data
        };

        stream.write_all(packet.to_buf().as_ref())?;

        bytes_send += block_size as u64;
        blocks_till_redraw -= 1;
    }

    // send FILE_END
    println!("checksum for file: {}", checksum);
    transport::send_slice(stream, Parsed::FileEnd(checksum).to_buf().as_ref())?;


    Ok(())
}

pub fn match_send(args: &Vec<String>) -> io::Result<crate::AppState> {
    // TODO customize port
    if args.len() < 3 {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }

    let recv: std::net::IpAddr = args[2].parse().map_err(|e| io::Error::from(io::ErrorKind::AddrNotAvailable))?;

    Ok(AppState::Send {
        to: SocketAddr::new(recv, 5123),
        files: SendFiles::Single("test.dat".to_owned()), // TODO parse files
    })

}

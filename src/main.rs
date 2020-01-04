use crossterm::style::{self, Colorize};
use crossterm::{cursor, queue};
use std::env;
use std::io::{self, stdout, Write};

mod recv;
mod send;
mod transport;
mod utils;

use utils::s_contains;
use std::path::PathBuf;
use std::fs::File;

pub enum AppState {
    Send {
        to: std::net::SocketAddr,
        files: send::SendFiles,
    },
    Recv,
    GenTestData(PathBuf, u64)
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();



    let bin_name = &args[0];

    let state = if s_contains(&args, "send") {
        send::match_send(&args)?
    } else if s_contains(&args, "recv") {
        AppState::Recv
    } else if s_contains(&args, "testgen") {
        if args.len() != 4 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }

        let fname = PathBuf::from(&args[2]);
        let size : u64 = args[3].parse().expect("Size in integer mb");

        AppState::GenTestData(fname, size)


    } else {
        println!("No mode specified!\nUsage:\n\t{binname} recv\t\t| waits for files\n\t{binname} send [addr ipv6 / ipv4] [list of files]", binname = bin_name);
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    };

    print_info(&state).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    match state {
        AppState::Send { .. } => {
            send::send(&state)?;
        }
        AppState::Recv => {
            recv::recv()?;
        }
        AppState::GenTestData(fname, size) => {
            let mut f = File::create(&fname)?;

            let mut rand_data: String = "asdf1234P.".repeat(size as usize * 410 );
            rand_data.truncate(4096 );

            for mb in 0..size {
                for p in 0..250 {
                    f.write_all(rand_data.as_bytes());
                }
                println!("Written {}/{} mb", mb + 1, size);
            }
        }
    }

    Ok(())
}

fn print_info(state: &AppState) -> crossterm::Result<()> {
    let mut stdout = stdout();
    queue!(
        stdout,
        style::PrintStyledContent("Simple File Share\n".magenta()),
        cursor::MoveDown(1)
    )?;

    match state {
        AppState::Send { to, files } => println!("Sending {} to {}", files, to),
        &AppState::Recv => println!("Waiting for files to receive"),
        &AppState::GenTestData(ref fname, ref size) => println!("Generating {:?} with {} mb", fname, size),
    }

    stdout.flush()?;
    Ok(())
}

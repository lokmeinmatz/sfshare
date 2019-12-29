use crossterm::style::{self, Colorize};
use crossterm::{cursor, queue};
use std::env;
use std::io::{self, stdout, Write};

mod recv;
mod send;
mod transport;
mod utils;

use utils::s_contains;

pub enum AppState {
    Send {
        to: std::net::SocketAddr,
        files: send::SendFiles,
    },
    Recv
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();



    let bin_name = &args[0];

    let state = if s_contains(&args, "send") {
        send::match_send(&args)?
    } else if s_contains(&args, "recv") {
        AppState::Recv
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
    }

    stdout.flush()?;
    Ok(())
}

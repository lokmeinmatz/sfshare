use clap::{App, Arg, SubCommand};
use std::io::{self, stdout, Write};
use crossterm::{queue, cursor};
use crossterm::style::{self, Colorize};

mod send;
mod recv;
mod transport;

pub enum AppState {
    Send{
        to: std::net::SocketAddr,
        files: send::SendFiles
    },
    Recv,
    Unknown(&'static str),
}


fn main() -> std::io::Result<()> {
    let app = App::new("SimpleFileShare")
        .version("0.1.0")
        .about("Can send files from one device to another")
        .author("lokmeinmatz (Matthias Kind)")
        .subcommand(SubCommand::with_name("send").arg(Arg::with_name("RECEIVER")
            .required(true)
            .index(1)
        ))
        .subcommand(SubCommand::with_name("recv"));

    let bin_name = app.get_bin_name().unwrap_or("./sfshare").to_owned();
    let matches = app.get_matches();

    let state = if let Some(send_matches) = matches.subcommand_matches("send") {
        send::match_send(send_matches)
    }
    else if let Some(recv_matches) = matches.subcommand_matches("recv") {
        AppState::Recv
    }
    else { AppState::Unknown("Unknown usage") };


    print_info(&state, &bin_name).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    match state {
        AppState::Send{..} => {
            send::send(&state)?;
        },
        AppState::Recv => {
            recv::recv()?;
        },
        AppState::Unknown(err) => return Err(io::Error::new(io::ErrorKind::InvalidInput, err))
    }

    Ok(())
}

fn print_info(state: &AppState, bin_name: &str) -> crossterm::Result<()> {
    let mut stdout = stdout();
        let cpos = cursor::position()?;
        queue!(stdout, 
            style::PrintStyledContent("Simple File Share\n".magenta()),
            cursor::MoveDown(1)
        )?;

        match state {
            AppState::Send{to, files} => println!("Sending {} to {}", files, to),
            &AppState::Recv => println!("Waiting for files to receive"),
            AppState::Unknown(err) => println!("Error: {}\n > Please type {} --help for usage!", err.white().on_red(), bin_name)
        }

        stdout.flush()?;
        Ok(())
}


/**
*/
pub mod flags {
    pub const PING: u8 = 0x10;
    pub const PONG: u8 = 0x20;
    pub const ACK_REQ : u8 = 0x11;
    pub const ACK_RES : u8 = 0x12;
}


#[derive(Debug)]
pub struct FileMeta {
    pub size: u64,
    pub path: PathBuf
}

impl FileMeta {
    pub fn from(path: &str) -> io::Result<FileMeta> {
        let meta = std::fs::metadata(path)?;

        Ok(FileMeta {
            size: meta.len(),
            path: PathBuf::from(path)
        })
    }

    pub fn to_byte_stream(&self) -> Vec<u8> {
        // structure : [4 byte file-id][8 byte file-size][2 byte name_len][name utf8]
        let mut file_name = self.path.file_name().and_then(|os| os.to_os_string().into_string().ok()).unwrap_or(String::new()).truncate(std::u16::MAX as usize);

        // limit file_name length to 2^16
        let file_name_bytes = file_name.as_bytes();

        let byte_size = 4 + 8 + 2 + file_name.len();
        let mut res = Vec::with_capacity(byte_size);

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hasher, Hash};

        let file_id: u32 = {
            let mut dh = DefaultHasher::new();
            file_name.hash(&mut dh);
            (dh.finish() % std::u32::MAX as u64) as u32
        };
        res.extend_from_slice(file_id.to_be_bytes());
        res.extend_from_slice(self.size.to_be_bytes());



        res
    }
}

#[derive(Debug)]
pub enum Parsed {
    Ping,
    Pong,
    AckReq(Vec<FileMeta>),
    AckRes(bool)
}

impl Parsed {
    pub fn to_buf(&self) -> Box<[u8]> {
        #[cfg(debug_assertions)]
            println!("{:?}.to_buf()", self);

        match self {
            Parsed::Ping => Box::new([flags::PING]),
            Parsed::Pong => Box::new([flags::PONG]),
            Parsed::AckReq(fm) => {
                let mut res = Vec::with_capacity(5 + 14 * fm.len());

                res.push(flags::ACK_REQ);
                let l_u32 = fm.len() as u32;
                res.extend_from_slice(l_u32.to_be_bytes());

                for f in fm {
                    res.extend(f.to_byte_stream())
                }

                res.into_boxed_slice()
            }
            //Parsed::AckRes(ack) => Box::new([flags::ACK_RES]), // TODO not finished
            _ => unimplemented!()
        }
    }
}

use std::io::{self, Error, ErrorKind, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::ffi::OsString;

pub  fn parse(stream: &mut TcpStream) -> io::Result<Parsed> {
    let mut reader : BufReader<& TcpStream> = BufReader::new(stream);

    let packet_type : u8 = {
        let mut d = [0u8];
        reader.read_exact(&mut d)?;
        d[0]
    };

    #[cfg(debug_assertions)]
    println!("packet_type: {:X}", packet_type);

    match packet_type {
        flags::PING => return Ok(Parsed::Ping),
        flags::PONG => return Ok(Parsed::Pong),
        flags::ACK_REQ => {
            let mut r_dat = [0u8, 0u8];
            reader.read_exact(&mut r_dat)?;
        }
        _ => {}
    }

    Err(Error::new(ErrorKind::InvalidData, "Can't parse stream"))
}

pub  fn send_slice<W : Write>(stream: &mut W, data: &[u8]) -> io::Result<()> {
    #[cfg(debug_assertions)]
    println!("sending {} bytes", data.len());
    stream.write_all(data)
}
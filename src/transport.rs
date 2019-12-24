/**
*/
pub mod flags {
    pub const PING: u8 = 0x01;
    pub const PONG: u8 = 0x02;
    pub const ACK_REQ: u8 = 0x11;
    pub const ACK_RES: u8 = 0x12;

}


#[derive(Debug)]
pub struct FileMeta {
    pub size: u64,
    pub id: u32,
    pub name: String,
    pub path: Option<PathBuf>,
}

impl FileMeta {
    pub fn from(path: &str) -> io::Result<FileMeta> {
        let meta = std::fs::metadata(path)?;
        let pbuf = PathBuf::from(path);

        let mut file_name = pbuf
            .file_name()
            .and_then(|os| os.to_os_string().into_string().ok())
            .unwrap_or(String::new());
        file_name.truncate(std::u16::MAX as usize);


        let file_id: u32 = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut dh = DefaultHasher::new();
            file_name.hash(&mut dh);
            (dh.finish() % std::u32::MAX as u64) as u32
        };


        Ok(FileMeta {
            size: meta.len(),
            path: Some(pbuf),
            id: file_id,
            name: file_name
        })
    }

    pub fn from_byte_stream<T: std::io::Read>(buf: &mut BufReader<T>) -> io::Result<FileMeta> {
        let f_id = {
            let mut b = [0u8; 4];
            buf.read_exact(&mut b)?;
            u32::from_be_bytes(b)
        };

        let f_size = {
            let mut b = [0u8; 8];
            buf.read_exact(&mut b)?;
            u64::from_be_bytes(b)
        };

        let f_name_len = {
            let mut b = [0u8; 2];
            buf.read_exact(&mut b)?;
            u16::from_be_bytes(b)
        };


        let f_name = {
            let mut collect = vec![0u8; f_name_len as usize];
            buf.read_exact(&mut collect)?;
            String::from_utf8(collect).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?
        };

        Ok(FileMeta{
            size: f_size,
            id: f_id,
            name: f_name,
            path: None
        })

    }

    pub fn to_byte_stream(&self) -> Vec<u8> {
        // structure : [4 byte file-id][8 byte file-size][2 byte name_len][name utf8]

        // limit file_name length to 2^16
        let file_name_bytes = self.name.as_bytes();

        assert!(file_name_bytes.len() <= std::u16::MAX as usize);

        let byte_size = 4 + 8 + 2 + file_name_bytes.len();
        let mut res = Vec::with_capacity(byte_size);

        
        res.extend_from_slice(&self.id.to_be_bytes());
        res.extend_from_slice(&self.size.to_be_bytes());
        res.extend_from_slice(&(file_name_bytes.len() as u16).to_be_bytes());
        res.extend_from_slice(file_name_bytes);
        assert_eq!(byte_size, res.len());

        res
    }
}

#[test]
fn test_bytestream() -> io::Result<()> {
    let fm = FileMeta::from("test.dat")?;

    let bs = fm.to_byte_stream();

    println!("{:?}", bs);
    let reconstruct = FileMeta::from_byte_stream(&mut BufReader::new(&bs[..]))?;

    assert_eq!(fm.id, reconstruct.id);
    assert_eq!(fm.name, reconstruct.name);
    assert_eq!(fm.size, reconstruct.size);
    Ok(())
}

#[derive(Debug)]
pub enum Parsed {
    Ping,
    Pong,
    AckReq(Vec<FileMeta>),
    AckRes(bool),
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
                res.extend_from_slice(&l_u32.to_be_bytes());

                for f in fm {
                    res.extend(f.to_byte_stream())
                }

                res.into_boxed_slice()
            }
            Parsed::AckRes(ack) => Box::new([flags::ACK_RES, (*ack) as u8]), // TODO not finished
            _ => unimplemented!(),
        }
    }
}

use std::io::{self, BufReader, Error, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

pub fn parse(stream: &mut TcpStream) -> io::Result<Parsed> {
    let mut reader: BufReader<&TcpStream> = BufReader::new(stream);

    let packet_type: u8 = {
        let mut d = [0u8];
        reader.read_exact(&mut d).expect("cant read packet_type");
        d[0]
    };

    #[cfg(debug_assertions)]
    println!("packet_type: 0x{:X}", packet_type);

    match packet_type {
        flags::PING => return Ok(Parsed::Ping),
        flags::PONG => return Ok(Parsed::Pong),
        flags::ACK_REQ => {
            let mut list_len = [0u8; 4];
            reader.read_exact(&mut list_len)?;

            let list_len = u32::from_be_bytes(list_len);
            let mut meta = Vec::with_capacity(list_len as usize);

            for i in 0..list_len {
                // parse next list item
                match FileMeta::from_byte_stream(&mut reader) {
                    Ok(fm) => meta.push(fm),
                    Err(_) => {
                        eprintln!("could not construct FileMeta for {}th file", i);
                        return Err(io::Error::from(ErrorKind::InvalidData));
                    },
                }
            }

            return Ok(Parsed::AckReq(meta));
        },
        flags::ACK_RES => {
            let mut b = [0u8];
            reader.read_exact(&mut b);
            return Ok(Parsed::AckRes(b[0] != 0));
        }
        _ => {}
    }

    Err(Error::new(ErrorKind::InvalidData, "Can't parse stream: unknown packet type"))
}

pub fn send_slice<W: Write>(stream: &mut W, data: &[u8]) -> io::Result<()> {
    #[cfg(debug_assertions)]
    println!("sending {} bytes", data.len());
    stream.write_all(data)
}

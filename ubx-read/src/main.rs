use ublox::*;
use clap::{Arg, Command};

use std::fs::File;
use flate2::read::GzDecoder;
use std::io::{BufReader, Read};

enum BufferedReader {
    Plain(BufReader<File>),
    Gzip(BufReader<GzDecoder<File>>),
}

impl BufferedReader {
    fn new(path: &str) -> Self {
        let fd = File::open(path)
            .expect(&format!("failed to open \"{}\"", path));
        if path.ends_with(".gz") {
            Self::Gzip(BufReader::new(GzDecoder::new(fd))) 
        } else {
            Self::Plain(BufReader::new(fd))
        }
    }
}

impl std::io::Read for BufferedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self {
            Self::Plain(ref mut h) => h.read(buf),
            Self::Gzip(ref mut h) => h.read(buf),
        }
    }
}

impl std::io::BufRead for BufferedReader {
    fn fill_buf(&mut self) -> Result<&[u8], std::io::Error> {
        match self {
            Self::Plain(ref mut bufreader) => bufreader.fill_buf(),
            Self::Gzip(ref mut bufreader) => bufreader.fill_buf(),
        }
    }
    fn consume(&mut self, s: usize) {
        match self {
            Self::Plain(ref mut bufreader) => bufreader.consume(s),
            Self::Gzip(ref mut bufreader) => bufreader.consume(s),
        }
    }
}

fn main() {
    let matches = Command::new("ubx-read")
        .author(clap::crate_authors!())
        .about("Read and parse UBX files")
        .arg_required_else_help(true)
        .arg(
            Arg::new("file")
                .value_name("FILE")
                .short('f')
                .long("fp")
                .required(true)
                .help("Local .ubx file path, can be gzip compressed.")
        )
        .get_matches();

    let fp = matches
        .get_one::<String>("file")
        .unwrap();

    let mut buf = [0; 2048];
    let mut parser = Parser::default();
    let mut reader = BufferedReader::new(fp);

    while let Ok(size) = reader.read(&mut buf) {
        if size > 0 {
            let mut it = parser.consume(&buf[..size]);
            while let Some(packet) = it.next() {
                println!("{:?}", packet);
            }
        }
    }
}

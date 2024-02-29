use clap::{value_parser, Arg, Command};
use serialport::{
    DataBits as SerialDataBits, FlowControl as SerialFlowControl, Parity as SerialParity,
    StopBits as SerialStopBits,
};
use std::time::Duration;
use ublox::*;

use std::fs::File;
use std::io::{Write, BufWriter};
use flate2::{write::GzEncoder, Compression};

enum BufferedWriter {
    Plain(BufWriter<File>),
    Gzip(BufWriter<GzEncoder<File>>),
}

impl BufferedWriter {
    fn new(path: &str) -> Self {
        let fd = File::create(path)
            .expect(&format!("failed to create file \"{}\"", path));
        if path.ends_with(".gz") {
            Self::Gzip(BufWriter::new(GzEncoder::new(fd, Compression::new(6))))
        } else {
            Self::Plain(BufWriter::new(fd))
        }
    }
}

impl std::io::Write for BufferedWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        match self {
            BufferedWriter::Gzip(ref mut writer) => writer.write(buf),
            BufferedWriter::Plain(ref mut writer) => writer.write(buf),
        }
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        match self {
            BufferedWriter::Gzip(ref mut writer) => writer.flush(),
            BufferedWriter::Plain(ref mut writer) => writer.flush(),
        }
    }
}

fn main() {
    let matches = Command::new("ubx-record")
        .author(clap::crate_authors!())
        .about("Record UBX files from your U-Blox receiver")
        .arg_required_else_help(true)
        .next_help_heading("Serial configuration")
        .arg(
            Arg::new("port")
                .value_name("PORT")
                .short('p')
                .long("port")
                .required(true)
                .help("Serial port to open"),
        )
        .arg(
            Arg::new("baud")
                .value_name("BAUD")
                .short('s')
                .long("baud")
                .required(false)
                .default_value("9600")
                .value_parser(value_parser!(u32))
                .help("Baud rate of the port to open"),
        )
        .arg(
            Arg::new("stop-bits")
                .long("stop-bits")
                .help("Number of stop bits to use for open port")
                .required(false)
                .value_parser(["1", "2"])
                .default_value("1"),
        )
        .arg(
            Arg::new("data-bits")
                .long("data-bits")
                .help("Number of data bits to use for open port")
                .required(false)
                .value_parser(["7", "8"])
                .default_value("8"),
        )
        .arg(
            Arg::new("parity")
                .long("parity")
                .help("Parity to use for open port")
                .required(false)
                .value_parser(["even", "odd"]),
        )
        .next_help_heading("Output file")
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .required(false)
                .value_name("FILE")
                .help("Output file name")
        )
        .subcommand(
            Command::new("configure")
                .about("Configure settings for specific UART/USB port")
                .arg(
                    Arg::new("port")
                        .long("select")
                        .required(true)
                        .default_value("usb")
                        .value_parser(value_parser!(String))
                        .long_help(
                            "Apply specific configuration to the selected port. Supported: usb, uart1, uart2.
Configuration includes: protocol in/out, data-bits, stop-bits, parity, baud-rate",
                        ),
                    )
                .arg(
                    Arg::new("cfg-baud")
                        .value_name("baud")
                        .long("baud")
                        .required(false)
                        .default_value("9600")
                        .value_parser(value_parser!(u32))
                        .help("Baud rate to set"),
                )
                .arg(
                    Arg::new("stop-bits")
                        .long("stop-bits")
                        .help("Number of stop bits to set")
                        .required(false)
                        .value_parser(["1", "2"])
                        .default_value("1"),
                )
                .arg(
                    Arg::new("data-bits")
                        .long("data-bits")
                        .help("Number of data bits to set")
                        .required(false)
                        .value_parser(["7", "8"])
                        .default_value("8"),
                )
                .arg(
                    Arg::new("parity")
                        .long("parity")
                        .help("Parity to set")
                        .required(false)
                        .value_parser(["even", "odd"]),
                )
        )
        .get_matches();

    let port = matches
        .get_one::<String>("port")
        .expect("Expected required 'port' cli argumnet");

    let baud = matches.get_one::<u32>("baud").cloned().unwrap_or(9600);
    let stop_bits = match matches.get_one::<String>("stop-bits").map(|s| s.as_str()) {
        Some("2") => SerialStopBits::Two,
        _ => SerialStopBits::One,
    };

    let data_bits = match matches.get_one::<String>("data-bits").map(|s| s.as_str()) {
        Some("7") => SerialDataBits::Seven,
        Some("8") => SerialDataBits::Eight,
        _ => {
            println!("Number of DataBits supported by uBlox is either 7 or 8");
            std::process::exit(1);
        },
    };

    let parity = match matches.get_one::<String>("parity").map(|s| s.as_str()) {
        Some("odd") => SerialParity::Even,
        Some("even") => SerialParity::Odd,
        _ => SerialParity::None,
    };

    let builder = serialport::new(port, baud)
        .stop_bits(stop_bits)
        .data_bits(data_bits)
        .timeout(Duration::from_millis(10))
        .parity(parity)
        .flow_control(SerialFlowControl::None);

    let port = builder.open().unwrap_or_else(|e| {
        eprintln!("Failed to open \"{}\". Error: {}", port, e);
        ::std::process::exit(1);
    });

    let mut device = Device::new(port);

    let path = match matches.get_one::<String>("output") {
        Some(output) => output.to_string(),
        None => "output.ubx.gz".to_string(),
    };

    let mut buf = [0; 2048];
    let mut writer = BufferedWriter::new(&path);

    // Parse cli for configuring specific uBlox UART port
    if let Some(("configure", sub_matches)) = matches.subcommand() {
        let (port_id, port_name) = match sub_matches.get_one::<String>("port").map(|s| s.as_str()) {
            Some(x) if x == "usb" => (Some(UartPortId::Usb), x),
            Some(x) if x == "uart1" => (Some(UartPortId::Uart1), x),
            Some(x) if x == "uart2" => (Some(UartPortId::Uart2), x),
            _ => (None, ""),
        };

        let baud = sub_matches.get_one::<u32>("baud").cloned().unwrap_or(9600);

        let stop_bits = match sub_matches
            .get_one::<String>("stop-bits")
            .map(|s| s.as_str())
        {
            Some("2") => SerialStopBits::Two,
            _ => SerialStopBits::One,
        };

        let data_bits = match sub_matches
            .get_one::<String>("data-bits")
            .map(|s| s.as_str())
        {
            Some("7") => SerialDataBits::Seven,
            Some("8") => SerialDataBits::Eight,
            _ => {
                println!("Number of DataBits supported by uBlox is either 7 or 8");
                std::process::exit(1);
            },
        };

        let parity = match sub_matches.get_one::<String>("parity").map(|s| s.as_str()) {
            Some("odd") => SerialParity::Even,
            Some("even") => SerialParity::Odd,
            _ => SerialParity::None,
        };
        let inproto = InProtoMask::UBLOX;
        let outproto = OutProtoMask::UBLOX;

        if let Some(port_id) = port_id {
            println!("Configuring '{}' port ...", port_name.to_uppercase());
            device
                .write_all(
                    &CfgPrtUartBuilder {
                        portid: port_id,
                        reserved0: 0,
                        tx_ready: 0,
                        mode: UartMode::new(
                            ublox_databits(data_bits),
                            ublox_parity(parity),
                            ublox_stopbits(stop_bits),
                        ),
                        baud_rate: baud,
                        in_proto_mask: inproto,
                        out_proto_mask: outproto,
                        flags: 0,
                        reserved5: 0,
                    }
                    .into_packet_bytes(),
                )
                .expect("Could not configure UBX-CFG-PRT-UART");
            device
                .wait_for_ack::<CfgPrtUart>()
                .expect("Could not acknowledge UBX-CFG-PRT-UART msg");
        }
    }

    // Enable the NavPvt packet
    // By setting 1 in the array below, we enable the NavPvt message for Uart1, Uart2 and USB
    // The other positions are for I2C, SPI, etc. Consult your device manual.
    println!("Enable UBX-NAV-PVT message on all serial ports: USB, UART1 and UART2 ...");
    device
        .write_all(
            &CfgMsgAllPortsBuilder::set_rate_for::<NavPvt>([0, 1, 1, 1, 0, 0]).into_packet_bytes(),
        )
        .expect("Could not configure ports for UBX-NAV-PVT");
    device
        .wait_for_ack::<CfgMsgAllPorts>()
        .expect("Could not acknowledge UBX-CFG-PRT-UART msg");

    // Send a packet request for the MonVer packet
    device
        .write_all(&UbxPacketRequest::request_for::<MonVer>().into_packet_bytes())
        .expect("Unable to write request/poll for UBX-MON-VER message");

    // Start streaming
    println!("uBlox device opened, streaming..");
    
    loop {
        if let Ok(size) = device.read_port(&mut buf) {
            if size > 0 {
                if writer.write_all(&buf).is_err() {
                    println!("failed dump into file");
                }
            }
        }
    }
}

fn ublox_stopbits(s: SerialStopBits) -> StopBits {
    // Seriaport crate doesn't support the other StopBits option of uBlox
    match s {
        SerialStopBits::One => StopBits::One,
        SerialStopBits::Two => StopBits::Two,
    }
}

fn ublox_databits(d: SerialDataBits) -> DataBits {
    match d {
        SerialDataBits::Seven => DataBits::Seven,
        SerialDataBits::Eight => DataBits::Eight,
        _ => {
            println!("uBlox only supports Seven or Eight data bits");
            DataBits::Eight
        },
    }
}

fn ublox_parity(v: SerialParity) -> Parity {
    match v {
        SerialParity::Even => Parity::Even,
        SerialParity::Odd => Parity::Odd,
        SerialParity::None => Parity::None,
    }
}

struct Device {
    port: Box<dyn serialport::SerialPort>,
    parser: Parser<Vec<u8>>,
}

impl Device {
    pub fn new(port: Box<dyn serialport::SerialPort>) -> Device {
        let parser = Parser::default();
        Device { port, parser }
    }

    pub fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.port.write_all(data)
    }

    pub fn update<T: FnMut(PacketRef)>(&mut self, mut cb: T) -> std::io::Result<()> {
        loop {
            const MAX_PAYLOAD_LEN: usize = 1240;
            let mut local_buf = [0; MAX_PAYLOAD_LEN];
            let nbytes = self.read_port(&mut local_buf)?;
            if nbytes == 0 {
                break;
            }

            // parser.consume adds the buffer to its internal buffer, and
            // returns an iterator-like object we can use to process the packets
            let mut it = self.parser.consume(&local_buf[..nbytes]);
            loop {
                match it.next() {
                    Some(Ok(packet)) => {
                        cb(packet);
                    },
                    Some(Err(_)) => {
                        // Received a malformed packet, ignore it
                    },
                    None => {
                        // We've eaten all the packets we have
                        break;
                    },
                }
            }
        }
        Ok(())
    }

    pub fn wait_for_ack<T: UbxPacketMeta>(&mut self) -> std::io::Result<()> {
        let mut found_packet = false;
        while !found_packet {
            self.update(|packet| {
                if let PacketRef::AckAck(ack) = packet {
                    if ack.class() == T::CLASS && ack.msg_id() == T::ID {
                        found_packet = true;
                    }
                }
            })?;
        }
        Ok(())
    }

    /// Reads the serial port, converting timeouts into "no data received"
    fn read_port(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self.port.read(output) {
            Ok(b) => Ok(b),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    Ok(0)
                } else {
                    Err(e)
                }
            },
        }
    }
}

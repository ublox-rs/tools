# U-Blox record

U-Blox record is a command line tool to generate UBX files from your U-Blox receiver.

UBX files contain raw UBX frames that you can later on parse with our [U-Blox protocol parser](https://github.com/ublox-rs/ublox).  
Compressed UBX files are one of the most compact GNSS format.

## Generate the tool

```bash
cargo build --release
```

## Command line

Generate UBX file:

```bash
./target/relase/ubx-record -p /dev/ttyUSB0 -s 9600 -o output.ubx
```

Generate gzip compressed UBX file:

```bash
./target/relase/ubx-record -p /dev/ttyUSB0 -s 9600 -o output.ubx.gz
```

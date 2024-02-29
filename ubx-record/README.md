# U-Blox record

U-Blox record is a command line tool to generate UBX files from your U-Blox receiver.

UBX files contain raw UBX frames that you can later on parse with our [U-Blox protocol parser](https://github.com/ublox-rs/ublox).  
Compressed UBX files are one of the most compact format to GNSS navigation.

## Generate the tool

```bash
cargo install ublox-record
```

## Command line

## Compressed files

Generate gzip files directly by enabling the `flate-rs` feature




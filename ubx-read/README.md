# U-Blox read

Read & parse an UBX file previously dumped from your U-Blox receiver.

## Generate the tool

```bash
cargo build --release
```

## Command line

Parse an UBX file:

```bash
./target/relase/ubx-read -f /tmp/test.ubx
```

Parse a gzip compressed UBX file:

```bash
./target/relase/ubx-read -f /tmp/test.ubx.gz
```

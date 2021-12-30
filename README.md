# traceroute

Just a simple Rust command line for executing "tcptraceroute" and collecting its output to InfluxDB.

## Linux on Raspberry Pi

Add Rust toolchain for building on Linux using `rustup` command.

```sh
$ rustup target add aarch64-unknown-linux-gnu
```

Also install `gcc` for compiling instead of the default one.

```sh
$ sudo apt install gcc-aarch64-linux-gnu
```

This project already configured for target "aarch64-unknown-linux-gnu" at `.cargo/config`.
For building on the specific target just type the following command.

```sh
$ cargo build --target aarch64-unknown-linux-gnu --release
```
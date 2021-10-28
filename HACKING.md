# Hacking

## Building

### Prerequisites
Make sure you the following softwares are installed:

- Rust toolchain (nightly) - Use [rustup](https://rustup.rs/) to install
- [cargo-binutils](https://crates.io/crates/cargo-binutils) and [rustfilt](https://crates.io/crates/rustfilt) crates
- Docker
  - Make sure `docker run hello-world` works without sudo.
- Python 3
- QEMU

#### macOS

```
$ brew install qemu gdb python3
$ brew install --cask docker
$ rustup override set nightly
$ rustup component add llvm-tools-preview rust-src
$ cargo install cargo-watch cargo-binutils rustfilt
```

### Make Commands

```
$ make                # Build the kernel (debug build)
$ make RELEASE=1      # Build the kernel (release build)
$ make run            # Run on QEMU
$ make run GUI=1      # Run on QEMU with an application window
                      # Note: Currently Kerla only use the serial port which
                      #       appears in the terminal running `make run`.
$ make run GDB=1      # Run on QEMU with GDB connection enabled
```

### How to Use
Once you boot the OS by `make run`, a Busybox shell shows up in your terminal. `initramfs/inittab.py` includes the boot script. Edit the file to run Dropbear SSH server instead of the shell.

# Quickstart

### Prerequisites
Make sure the following software is installed:

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
```

#### Ubuntu

```
$ sudo apt install qemu-system gdb python3
```

## Building
```
$ git clone https://github.com/nuta/kerla && cd kerla
$ rustup override set nightly
$ rustup component add llvm-tools-preview rust-src
$ cargo install cargo-watch cargo-binutils rustfilt
$ make
```

### Make Commands

```
$ make                # Build OS (debug build)
$ make RELEASE=1      # Build OS (release build)
$ make run            # Run on QEMU
$ make run LOG=trace  # Run on QEMU w/ trace messages enabled
$ make run GDB=1      # Run on QEMU with GDB connection enabled (listens on localhost:7789)
```

### Running OS on QEMU
Once you boot the OS with `make run`, a Busybox shell shows up in your terminal.

The terminal running QEMU emulates a serial port connected to Kerla. What you type on
the terminal will be sent to the Kerla and the foreground process.

#### QEMU Commands
Type <kbd>Ctrl + A</kbd> then <kbd>C</kbd> to switch the terminal into the QEMU monitor mode. The useful commands are:

- `q`: Quit the emulator.
- `info registers`: Dump the CPU registers.
- `info qtree`: List peripherals connected to the VM.

### Startup Script
Edit `initramfs/inittab.py` to run shell scripts automatically.

### Running a Docker image
You can run a Docker image as a root file system (not as a container!) on Kerla Kernel instead of our initramfs built from `initramfs` directory.

To run [nuta/helloworld](https://hub.docker.com/r/nuta/helloworld), type:

```
$ make IMAGE=nuta/helloworld run
```

This feature is in a very early stage and I guess **almost all images out there won't work** because:

- They tend to be too large to be embedded into the kernel image.
- They might use unimplemented features (e.g. position-independent executables used in Alpine Linux).

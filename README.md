Penguin Kernel
==============

Rwrite Linux Kenrel in Rust *just for fun*!

## Road Map

- [ ] Page allocator
- [ ] Initramfs
- [ ] Context switching
- [ ] System calls
- [ ] tty
- [ ] exec, fork, wait, exit
- [ ] open, read, write, close
- [ ] signal
- [ ] cgroups
- [ ] TCP/IP protocol stack ([smoltcp](https://github.com/smoltcp-rs/smoltcp) or [Fuchsia's Netstack3](https://fuchsia.dev/fuchsia-src/contribute/contributing_to_netstack3))
- [ ] File system (ext4?)

## Prerequisites
- Docker Engine

```
$ brew install qemu gdb python3
```

```
$ rustup override set nightly
$ rustup component add llvm-tools-preview
$ cargo install cargo-binutils cargo-watch rustfilt
```

## Building
```
$ make                # Build the kernel (debug build)
$ make RELEASE=1      # Build the kernel (release build)
$ make run            # Run on QEMU
$ make run GUI=1      # Run on QEMU with GUI enabled
$ make run GDB=1      # Run on QEMU with GDB connection enabled
```

## License
CC0 or MIT. Choose whichever you prefer.

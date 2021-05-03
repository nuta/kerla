Penguin Kernel
==============

Rwrite Linux Kenrel in Rust *just for fun*!

## Running a Docker Image (experimental)
You can run a Docker image instead of our initramfs built from `packages` directory.

For example, to [nuta/helloworld](https://hub.docker.com/r/nuta/helloworld) image ([Dockerfile](https://gist.github.com/nuta/4c9ecd0d1a401dc5be88095bea5a991a)), try the following command:

```
$ make IMAGE=nuta/helloworld run
...
[   0.029] syscall: execve(439398, 4393b8, 4393c8, 8, 2f2f2f2f2f2f2f2f, 8080808080808080)
[   0.030] syscall: arch_prctl(1002, 4055d8, 0, 20000, 0, ff)
[   0.031] syscall: set_tid_address(4057f0, 4055d8, 0, 20000, 0, ff)
[   0.033] syscall: ioctl(1, 5413, 9ffffeed0, 1, 405040, 9ffffeef7)

 _          _ _                            _     _ _
| |__   ___| | | ___   __      _____  _ __| | __| | |
| '_ \ / _ \ | |/ _ \  \ \ /\ / / _ \| '__| |/ _` | |
| | | |  __/ | | (_) |  \ V  V / (_) | |  | | (_| |_|
|_| |_|\___|_|_|\___/    \_/\_/ \___/|_|  |_|\__,_(_)
```

This feature is in the very early stage and I guess **almost all images out there won't work** because:

- They tend to be too large to embed into the kernel image.
- They might use unimplemented features (e.g. position-independent executables used in Alpine Linux).

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

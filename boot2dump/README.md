# boot2dump

[![crates.io](https://img.shields.io/crates/v/boot2dump.svg)](https://crates.io/crates/boot2dump)
[![docs.rs](https://docs.rs/boot2dump/badge.svg)](https://docs.rs/boot2dump)

A tiny operating system which takes a filename and memory buffer, saves it into
the disk, and reboots the computer. It's initially designed for collecting kernel
crash logs from [Kerla](https://kerla.dev) running on cloud.

## Prerequisites
Boot2dump assumes the following prerequisites:

- The CPU is x86_64 and is in the 64-bit mode.
- The file system is ext4 and its on a virtio-blk device.
- A **sufficiently large** file for the crash log (e.g. in the following example, `kerla.dump`)
  already exists in the **root directory**.
- The starting address of the boot2dump **image is aligned to 4096**.
- Virtual addresses starting `0xffff_8000_0000_0000` are straight mapped into
  from the physical addresses 0 (i.e. `0xffff_8000_0001_0000` points to `0x1_0000`).
  It should cover the memory pages where boot2dump image exist.

## How to Use

1. Create a file to save the crash log.

```
$ dd if=/dev/zero of=/kerla.dump bs=1024 count=64
```

2. Embed `boo2dump.bin` and call it from your panic handler. Make sure **boot2dump image (`__boot2dump`) is aligned to 4096**.

```c
// Embed boot2dump.bin using llvm-objcopy or something.
extern char __boot2dump[];
typedef void (*boot2dump_entry_t)(const char *filename, unsigned char *buf,
                                  unsigned long long len);

void your_panic_handler(void) {
    // The panic log to be saved into a file.
    unsigned char log[] = "Hello World from the panic handler!";

    // Jump into boot2dump. It won't return.
    boot2dump_entry_t entry = (boot2dump_entry_t) __boot2dump;
    entry("kerla.dump", log, sizeof(log));

    // Unreachable here!
}
```

## License

See LICENSE.md.

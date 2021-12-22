# Debugging 101

## Enable `trace` messages

See [logging](../logging).

## QEMU exits without a kernel panic message

Use QEMU's logging feature (`-d int,cpu_reset`) to print the reason.

```
qemu-system-x86_64 -d int,cpu_reset
```

## Debugging Device Drivers

- Build QEMU from the source. QEMU device emulation tends to provide a [DEBUG macro](https://github.com/qemu/qemu/blob/8c5f94cd4182753959c8be8de415120dc879d8f0/hw/net/e1000.c#L47) to enable debug messages. Also, adding `printf`s by your own helps a lot.
- Use [QEMU's tracing feature](https://qemu-project.gitlab.io/qemu/devel/tracing.html).

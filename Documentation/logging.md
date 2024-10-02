# Logging

By default, the kernel doesn't print `trace!` or `debug!` log messages.

Similar to `$RUST_LOG`, Kerla supports controlling logging through `LOG=` argument in a `make` command:

```
make run LOG=trace               # Enable all trace messages.
make run LOG="kernel::fs=trace"  # Enable traces messages in `kerla_kernel::fs`.
                                 # "kerla_" prefix can be omitted.
```

## Using the secondary serial port

If the kernel messages are noisy, you can use the secondary serial port to forward them into a separate file:

```
make run \
    LOG=trace LOG_SERIAL="chardev:uart1" \
    QEMU_ARGS="-chardev file,id=uart1,path=/tmp/kerla-debug.log,logappend=on"
```

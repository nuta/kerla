# DigitalOcean

## Observations

- Two virtio-net (legacy) devices are attached. For public IP address and private network respectively.
- The disk will be attached as a virtio-blk (legacy) device.
- DHCP is available if a custom image for `Unknown OS` is used.
  - It's disabled for Droplets using major operating systems like Ubuntu and FreeBSD.
- No serial port support.
  - You can see the kernel messages from *Recovery Console*.

## Creating a Custom Image

DigitalOcean supports using a custom OS image. You can create a disk image
by `tools/create-qcow2.sh` (requires Linux).

```
$ make RELEASE=1
$ ./tools/create-qcow2.sh kerla.x64.elf kerla.qcow2
```

# Kernel Parameters

Kernel parameters allows you to configure Kerla at the boot time.

## Available Parameters

| Name                 | Description                                                                                                                     | Example                         |
|----------------------|---------------------------------------------------------------------------------------------------------------------------------|---------------------------------|
| `log`                | Logging configuration (see [Logging](logging)).                                                                                 | `log=trace`                     |
| `serial1`            | If it's on, kernel log messages are sent to the secondary serial port (see [Logging](logging)).                                 | `serial1=on`                    |
| `dhcp`               | If it's off, the in-kernel DHCP client won't start.                                                                             | `dhcp=off`                      |
| `ip4`                | A static IPv4 address with the network prefix length.                                                                           | `ip4=10.0.0.123/24`             |
| `gateway_ip4`        | A static gateway IPv4 address.                                                                                                  | `gateway_ip4=10.0.0.1`          |
| `pci`                | If it's off, PCI devices are not discovered.                                                                                    | `pci=off`                       |
| `pci_device`         | PCI devices (`bus:slot`) recognized by Kerla. Multiple parameters are accepted. If it's not given, all PCI devices are allowed. | `pci_device=0:1`                |
| `virtio_mmio.device` | The virtio devices connected over MMIO. Multiple parameters are accepted.                                                       | `virtio_mmio.device=@0xf000:12` |

## How to Set Kernel Parameters

### make

In `make`, you can specify kernel parameters through `CMDLINE=`:

```
make run CMDLINE="dhcp=off"
```

### GRUB2

In GRUB2, append kernel parameters after the kernel image path:

```
menuentry "Kerla" {
    multiboot2 /boot/kerla.elf dhcp=off
}
```

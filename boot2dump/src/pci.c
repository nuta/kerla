#include "pci.h"
#include <printf.h>

static uint32_t read32(uint8_t bus, uint8_t slot, uint16_t offset) {
    ASSERT(IS_ALIGNED(offset, 4));
    uint32_t addr = (1UL << 31) | (bus << 16) | (slot << 11) | offset;
    ioport_write32(PCI_IOPORT_ADDR, addr);
    return ioport_read32(PCI_IOPORT_DATA);
}

static uint8_t read8(uint8_t bus, uint8_t slot, uint16_t offset) {
    uint32_t value = read32(bus, slot, offset & 0xfffc);
    return (value >> ((offset & 0x03) * 8)) & 0xff;
}

static uint16_t read16(uint8_t bus, uint8_t slot, uint16_t offset) {
    uint32_t value = read32(bus, slot, offset & 0xfffc);
    return (value >> ((offset & 0x03) * 8)) & 0xffff;
}

static void write32(uint8_t bus, uint8_t slot, uint16_t offset,
                    uint32_t value) {
    ASSERT(IS_ALIGNED(offset, 4));
    uint32_t addr = (1UL << 31) | (bus << 16) | (slot << 11) | offset;
    ioport_write32(PCI_IOPORT_ADDR, addr);
    ioport_write32(PCI_IOPORT_DATA, value);
}

void pci_enable_bus_master(struct pci_device *dev) {
    uint32_t value = read32(dev->bus, dev->slot, PCI_CONFIG_COMMAND) | (1 << 2);
    write32(dev->bus, dev->slot, PCI_CONFIG_COMMAND, value);
}

error_t pci_find_device(bool (*callback)(uint16_t vendor, uint16_t device),
                        struct pci_device *dev) {
    for (int bus = 0; bus <= 255; bus++) {
        for (int slot = 0; slot < 32; slot++) {
            uint16_t vendor = read16(bus, slot, PCI_CONFIG_VENDOR_ID);
            uint16_t device = read16(bus, slot, PCI_CONFIG_DEVICE_ID);
            if (vendor == 0xffff) {
                continue;
            }

            if (callback(vendor, device)) {
                dev->bus = bus;
                dev->slot = slot;
                return OK;
            }
        }
    }

    return ERR_NOT_FOUND;
}

uint32_t pci_read_config(struct pci_device *dev, unsigned offset,
                         unsigned size) {
    switch (size) {
        case 1:
            return read8(dev->bus, dev->slot, offset);
        case 2:
            return read16(dev->bus, dev->slot, offset);
        case 4:
            return read32(dev->bus, dev->slot, offset);
        default:
            return 0;
    }
}

void pci_write_config(struct pci_device *dev, unsigned offset, unsigned size,
                      uint32_t value) {
    switch (size) {
        case 1:
            //            write8(dev->bus, dev->slot, offset, value);
            NYI();
            break;
        case 2:
            //            write16(dev->bus, dev->slot, offset, value);
            NYI();
            break;
        case 4:
            write32(dev->bus, dev->slot, offset, value);
            break;
    }
}

#ifndef __PCI_H__
#define __PCI_H__

#include <types.h>

#define PCI_IOPORT_BASE 0x0cf8
#define PCI_IOPORT_ADDR (PCI_IOPORT_BASE + 0x00)
#define PCI_IOPORT_DATA (PCI_IOPORT_BASE + 0x04)
#define PCI_ANY         0

#define PCI_CONFIG_VENDOR_ID 0x00
#define PCI_CONFIG_DEVICE_ID 0x02
#define PCI_CONFIG_COMMAND   0x04
#define PCI_CONFIG_BAR0      0x10
#define PCI_CONFIG_INTR_LINE 0x3c

struct pci_device {
    uint8_t bus;
    uint8_t slot;
};

void pci_enable_bus_master(struct pci_device *dev);
error_t pci_find_device(bool (*callback)(uint16_t vendor, uint16_t device),
                        struct pci_device *dev);
uint32_t pci_read_config(struct pci_device *dev, unsigned offset,
                         unsigned size);
void pci_write_config(struct pci_device *dev, unsigned offset, unsigned size,
                      uint32_t value);
void pci_init(void);

#endif

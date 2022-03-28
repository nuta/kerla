#include "virtio.h"
#include <endian.h>
#include <page_alloc.h>
#include <pci.h>
#include <printf.h>
#include <string.h>
#include <types.h>

/// The maximum number of virtqueues.
#define NUM_VIRTQS_MAX 8

static uint16_t port_base;
static struct virtio_virtq virtqs[NUM_VIRTQS_MAX];

static uint8_t read_device_status(void) {
    return ioport_read8(port_base + VIRTIO_REG_DEVICE_STATUS);
}

static void write_device_status(uint8_t value) {
    ioport_write8(port_base + VIRTIO_REG_DEVICE_STATUS, value);
}

static uint64_t read_device_features(void) {
    return ioport_read32(port_base + VIRTIO_REG_DEVICE_FEATS);
}

/// Reads the ISR status and de-assert an interrupt
/// ("4.1.4.5 ISR status capability").
static uint8_t read_isr_status(void) {
    return ioport_read8(port_base + VIRTIO_REG_ISR_STATUS);
}

/// Returns the number of descriptors in total in the queue.
static uint16_t virtq_num_descs(void) {
    return ioport_read16(port_base + VIRTIO_REG_NUM_DESCS);
}

/// Returns the `index`-th virtqueue.
static struct virtio_virtq *virtq_get(unsigned index) {
    return &virtqs[index];
}

/// Notifies the device that the queue contains a descriptor it needs to
/// process.
static void virtq_notify(struct virtio_virtq *vq) {
    mb();
    ioport_write16(port_base + VIRTIO_REG_QUEUE_NOTIFY, vq->index);
}

/// Selects the current virtqueue in the common config.
static void virtq_select(unsigned index) {
    ioport_write16(port_base + VIRTIO_REG_QUEUE_SELECT, index);
}

/// Initializes a virtqueue.
static void virtq_init(unsigned index) {
    virtq_select(index);

    size_t num_descs = virtq_num_descs();
    ASSERT(num_descs <= 4096 && "too large queue size");

    offset_t avail_ring_off = sizeof(struct virtq_desc) * num_descs;
    size_t avail_ring_size = sizeof(uint16_t) * (3 + num_descs);
    offset_t used_ring_off =
        ALIGN_UP(avail_ring_off + avail_ring_size, PAGE_SIZE);
    size_t used_ring_size =
        sizeof(uint16_t) * 3 + sizeof(struct virtq_used_elem) * num_descs;
    size_t virtq_size = used_ring_off + ALIGN_UP(used_ring_size, PAGE_SIZE);

    vaddr_t virtq_base =
        (vaddr_t) page_alloc(ALIGN_UP(virtq_size, PAGE_SIZE) / PAGE_SIZE, true);

    struct virtio_virtq *vq = &virtqs[index];
    vq->index = index;
    vq->num_descs = num_descs;
    vq->legacy.next_avail_index = 0;
    vq->legacy.last_used_index = 0;
    vq->legacy.descs = (struct virtq_desc *) virtq_base;
    vq->legacy.avail = (struct virtq_avail *) (virtq_base + avail_ring_off);
    vq->legacy.used = (struct virtq_used *) (virtq_base + used_ring_off);

    // Add descriptors into the free list.
    vq->legacy.free_head = 0;
    vq->legacy.num_free_descs = num_descs;
    for (size_t i = 0; i < num_descs; i++) {
        vq->legacy.descs[i].next = (i + 1 == num_descs) ? 0 : i + 1;
    }

    paddr_t paddr = vaddr2paddr(virtq_base);
    ASSERT(IS_ALIGNED(paddr, PAGE_SIZE));
    ioport_write32(port_base + VIRTIO_REG_QUEUE_ADDR_PFN, paddr / PAGE_SIZE);
}

static void activate(void) {
    write_device_status(read_device_status() | VIRTIO_STATUS_DRIVER_OK);
}

/// Enqueues a chain of descriptors into the virtq. Don't forget to call
/// `notify` to start processing the enqueued request.
static error_t virtq_push(struct virtio_virtq *vq,
                          struct virtio_chain_entry *chain, int n) {
    DEBUG_ASSERT(n > 0);
    if (n > vq->legacy.num_free_descs) {
        // Try freeing used descriptors.
        while (vq->legacy.last_used_index != vq->legacy.used->index) {
            struct virtq_used_elem *used_elem =
                &vq->legacy.used
                     ->ring[vq->legacy.last_used_index % vq->num_descs];

            // Count the number of descriptors in the chain.
            int num_freed = 0;
            int prev_free_head = vq->legacy.free_head;
            int next_desc_index = used_elem->id;
            while (true) {
                struct virtq_desc *desc = &vq->legacy.descs[next_desc_index];
                num_freed++;

                if ((desc->flags & VIRTQ_DESC_F_NEXT) == 0) {
                    desc->next = prev_free_head;
                    break;
                }

                next_desc_index = desc->next;
            }

            // Enqueue the chain back into the free list.
            vq->legacy.free_head = used_elem->id;
            vq->legacy.num_free_descs += num_freed;
            vq->legacy.last_used_index++;
        }
    }

    if (n > vq->legacy.num_free_descs) {
        PANIC("virtq_push: run out of descriptors");
    }

    int head_index = vq->legacy.free_head;
    int desc_index = head_index;
    struct virtq_desc *desc = NULL;
    for (int i = 0; i < n; i++) {
        struct virtio_chain_entry *e = &chain[i];
        desc = &vq->legacy.descs[desc_index];
        desc->addr = into_le64(e->addr);
        desc->len = into_le32(e->len);
        desc->flags =
            (e->device_writable ? VIRTQ_DESC_F_WRITE : 0) | VIRTQ_DESC_F_NEXT;
        desc_index = desc->next;
    }

    // Update the last entry in the chain.
    DEBUG_ASSERT(desc != NULL);
    int unused_next = desc->next;
    desc->next = 0;
    desc->flags &= ~VIRTQ_DESC_F_NEXT;

    vq->legacy.free_head = unused_next;
    vq->legacy.num_free_descs -= n;

    // Append the chain into the avail ring.
    vq->legacy.avail->ring[vq->legacy.avail->index % vq->num_descs] =
        head_index;
    mb();
    vq->legacy.avail->index++;
    return OK;
}

/// Pops a descriptor chain processed by the device. Returns the number of
/// descriptors in the chain and fills `chain` with the popped descriptors.
///
/// If no chains in the used ring, it returns ERR_EMPTY.
static int virtq_pop(struct virtio_virtq *vq, struct virtio_chain_entry *chain,
                     int n, size_t *total_len) {
    if (vq->legacy.last_used_index == vq->legacy.used->index) {
        return ERR_EMPTY;
    }

    struct virtq_used_elem *used_elem =
        &vq->legacy.used->ring[vq->legacy.last_used_index % vq->num_descs];

    *total_len = used_elem->len;
    int next_desc_index = used_elem->id;
    struct virtq_desc *desc = NULL;
    int num_popped = 0;
    while (num_popped < n) {
        desc = &vq->legacy.descs[next_desc_index];
        chain[num_popped].addr = desc->addr;
        chain[num_popped].len = desc->len;
        chain[num_popped].device_writable =
            (desc->flags & VIRTQ_DESC_F_WRITE) != 0;

        num_popped++;

        bool has_next = (desc->flags & VIRTQ_DESC_F_NEXT) != 0;
        if (!has_next) {
            break;
        }

        if (num_popped >= n && has_next) {
            // `n` is too short.
            return ERR_NO_MEMORY;
        }

        next_desc_index = desc->next;
    }

    // Prepend the popped descriptors into the free list.
    DEBUG_ASSERT(desc != NULL);
    desc->next = vq->legacy.free_head;
    vq->legacy.free_head = used_elem->id;
    vq->legacy.num_free_descs += num_popped;

    vq->legacy.last_used_index++;
    return num_popped;
}

/// Checks and enables features. It aborts if any of the features is not
/// supported.
static void negotiate_feature(uint64_t features) {
    // Abort if the device does not support features we need.
    ASSERT((read_device_features() & features) == features);
    ioport_write32(port_base + VIRTIO_REG_DRIVER_FEATS, features);
    write_device_status(read_device_status() | VIRTIO_STATUS_FEAT_OK);
    ASSERT((read_device_status() & VIRTIO_STATUS_FEAT_OK) != 0);
}

static uint64_t read_device_config(offset_t offset, size_t size) {
    return ioport_read8(port_base + VIRTIO_REG_DEVICE_CONFIG_BASE + offset);
}

static struct virtio_ops virtio_legacy_ops = {
    .read_device_features = read_device_features,
    .negotiate_feature = negotiate_feature,
    .read_device_config = read_device_config,
    .activate = activate,
    .read_isr_status = read_isr_status,
    .virtq_init = virtq_init,
    .virtq_get = virtq_get,
    .virtq_push = virtq_push,
    .virtq_pop = virtq_pop,
    .virtq_notify = virtq_notify,
};

/// Looks for and initializes a virtio device with the given device type. It
/// sets the IRQ vector to `irq` on success.
error_t virtio_legacy_find_device(bool (*pci_find)(uint16_t vendor,
                                                   uint16_t device),
                                  struct virtio_ops **ops) {
    // Search the PCI bus for a virtio device...
    struct pci_device pci_dev;
    error_t err;
    if ((err = pci_find_device(pci_find, &pci_dev)) != OK) {
        return err;
    }

    uint32_t bar0 = pci_read_config(&pci_dev, 0x10, sizeof(uint32_t));
    ASSERT((bar0 & 1) == 1 && "BAR#0 should be io-mapped");

    port_base = bar0 & ~0b11;

    // Enable PCI bus master.
    pci_enable_bus_master(&pci_dev);

    // "3.1.1 Driver Requirements: Device Initialization"
    write_device_status(0);  // Reset the device.
    write_device_status(read_device_status() | VIRTIO_STATUS_ACK);
    write_device_status(read_device_status() | VIRTIO_STATUS_DRIVER);

    TRACE("found a virtio-legacy device");
    *ops = &virtio_legacy_ops;
    return OK;
}

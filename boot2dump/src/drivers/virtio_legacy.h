#ifndef __VIRTIO_LEGACY_H__
#define __VIRTIO_LEGACY_H__

#include <types.h>

#define VIRTIO_REG_DEVICE_FEATS       0x00
#define VIRTIO_REG_DRIVER_FEATS       0x04
#define VIRTIO_REG_QUEUE_ADDR_PFN     0x08
#define VIRTIO_REG_NUM_DESCS          0x0c
#define VIRTIO_REG_QUEUE_SELECT       0x0e
#define VIRTIO_REG_QUEUE_NOTIFY       0x10
#define VIRTIO_REG_DEVICE_STATUS      0x12
#define VIRTIO_REG_ISR_STATUS         0x13
#define VIRTIO_REG_DEVICE_CONFIG_BASE 0x14

#define DEVICE_STATUS_ACKNOWLEDGE        1
#define DEVICE_STATUS_DRIVER             2
#define DEVICE_STATUS_DRIVER_OK          4
#define DEVICE_STATUS_FEATURES_OK        8
#define DEVICE_STATUS_DEVICE_NEEDS_RESET 64
#define DEVICE_STATUS_FAILED             128

struct virtq_desc {
    uint64_t addr;
    uint32_t len;
    uint16_t flags;
    uint16_t next;
} __packed;

struct virtq_avail {
    uint16_t flags;
    uint16_t index;
    uint16_t ring[];
} __packed;

/*uint32_t is used here for ids for padding reasons. */
struct virtq_used_elem {
    uint32_t id;
    uint32_t len;
} __packed;

struct virtq_used {
    uint16_t flags;
    uint16_t index;
    struct virtq_used_elem ring[];
} __packed;

struct virtio_virtq_legacy {
    int next_avail_index;
    int last_used_index;
    int free_head;
    int num_free_descs;
    struct virtq_desc *descs;
    struct virtq_avail *avail;
    struct virtq_used *used;
};

struct virtio_ops;
error_t virtio_legacy_find_device(bool (*pci_find)(uint16_t vendor,
                                                   uint16_t device),
                                  struct virtio_ops **ops);

#endif

#include "virtio_blk.h"
#include <disk.h>
#include <drivers/virtio.h>
#include <page_alloc.h>
#include <pci.h>
#include <printf.h>
#include <string.h>

static struct virtio_ops *virtio;
static struct virtio_virtq *virtq;
static struct virtio_blk_req_header *request_headers;
static volatile uint8_t *request_statuses;
static volatile bool *in_progress;

static bool pci_find(uint16_t vendor, uint16_t device) {
    return vendor == 0x1af4 && device == 0x1001;
}

static void wait_for_completion(struct virtio_blk_req_header *header,
                                volatile uint8_t *status) {
    // Wait until the device writes the status...
    while (*status == IN_PROGRESS_STATUS)
        ;

    switch (*status) {
        case VIRTIO_BLK_S_OK:
            break;
        case VIRTIO_BLK_S_IOERR:
            PANIC("virtio_blk: VIRTIO_BLK_S_IOERR errror (lba=%u)",
                  header->sector);
            break;
        case VIRTIO_BLK_S_UNSUPP:
            PANIC("virtio_blk: VIRTIO_BLK_S_UNSUPP errror (lba=%u)",
                  header->sector);
            break;
        default:
            PANIC("virtio_blk: unknown status 0x%x", *status);
            break;
    }
}

void disk_read_sectors(uint64_t lba, uint8_t *buf, size_t num_sectors) {
    // struct virtio_blk_req {
    //     // chain[0]: device readable
    //     le32 type;
    //     le32 reserved;
    //     le64 sector;
    //
    //     // chain[1]: device writable
    //     u8 data[][512];
    //
    //     // chain[2]: device writable
    //     u8 status;
    // };
    struct virtio_blk_req_header header;
    volatile uint8_t status;

    header.type = VIRTIO_BLK_T_IN;
    header.sector = lba;
    status = IN_PROGRESS_STATUS;

    struct virtio_chain_entry chain[3];
    chain[0].addr = ptr2paddr(&header);
    chain[0].len = sizeof(header);
    chain[0].device_writable = false;
    chain[1].addr = ptr2paddr(buf);
    chain[1].len = num_sectors * SECTOR_SIZE;
    chain[1].device_writable = true;
    chain[2].addr = ptr2paddr((void *) &status);
    chain[2].len = sizeof(status);
    chain[2].device_writable = true;

    virtio->virtq_push(virtq, chain, 3);
    virtio->virtq_notify(virtq);

    // We need to wait until the request is complete because `header` and
    // `status` are in the stack.
    wait_for_completion(&header, &status);
}

void disk_write_sectors(uint64_t lba, uint8_t *buf, size_t num_sectors) {
    // struct virtio_blk_req {
    //     // chain[0]: device readable
    //     le32 type;
    //     le32 reserved;
    //     le64 sector;
    //
    //     // chain[1]: device readable
    //     u8 data[][512];
    //
    //     // chain[2]: device writable
    //     u8 status;
    // };
    struct virtio_blk_req_header header;
    volatile uint8_t status;

    header.type = VIRTIO_BLK_T_OUT;
    header.sector = lba;
    status = IN_PROGRESS_STATUS;

    struct virtio_chain_entry chain[3];
    chain[0].addr = ptr2paddr(&header);
    chain[0].len = sizeof(header);
    chain[0].device_writable = false;
    chain[1].addr = ptr2paddr(buf);
    chain[1].len = num_sectors * SECTOR_SIZE;
    chain[1].device_writable = false;
    chain[2].addr = ptr2paddr((void *) &status);
    chain[2].len = sizeof(status);
    chain[2].device_writable = true;

    virtio->virtq_push(virtq, chain, 3);
    virtio->virtq_notify(virtq);

    // We need to wait until the request  because `header` and
    // `status` are in the stack.
    wait_for_completion(&header, &status);
}

void disk_write_to_sector_list(list_t *sector_list, uint8_t *buf, size_t len) {
    LIST_FOR_EACH (e, sector_list, struct sector_list_entry, next) {
        if (len < e->num_sectors * SECTOR_SIZE) {
            uint8_t tmp[SECTOR_SIZE];
            memcpy(tmp, buf, len);
            memset(tmp + len, 0, SECTOR_SIZE - len);
            INFO("lba=%u (%x), len=%u", e->lba, e->lba * SECTOR_SIZE, len);
            disk_write_sectors(e->lba, tmp, 1);
            break;
        }

        disk_write_sectors(e->lba, buf, e->num_sectors);
        buf += e->num_sectors * SECTOR_SIZE;
        len -= e->num_sectors * SECTOR_SIZE;
    }
}

void disk_init(void) {
    if (virtio_legacy_find_device(pci_find, &virtio) != OK) {
        PANIC("failed to find a virtio block device");
    }

    virtio->negotiate_feature(0);
    virtio->virtq_init(VIRTIO_BLK_REQUEST_QUEUE);
    virtq = virtio->virtq_get(VIRTIO_BLK_REQUEST_QUEUE);

    request_headers = page_alloc(
        ALIGN_UP(sizeof(struct virtio_blk_req_header) * virtq->num_descs,
                 PAGE_SIZE)
            / PAGE_SIZE,
        false);
    request_statuses = page_alloc(
        ALIGN_UP(sizeof(uint8_t) * virtq->num_descs, PAGE_SIZE) / PAGE_SIZE,
        true);
    in_progress = page_alloc(
        ALIGN_UP(sizeof(bool) * virtq->num_descs, PAGE_SIZE) / PAGE_SIZE, true);

    INFO("initialized a virtio block device");
}

#ifndef __VIRTIO_BLK_H__
#define __VIRTIO_BLK_H__

#include <types.h>

#define VIRTIO_BLK_REQUEST_QUEUE 0

#define VIRTIO_BLK_T_IN  0
#define VIRTIO_BLK_T_OUT 1

#define VIRTIO_BLK_S_OK     0
#define VIRTIO_BLK_S_IOERR  1
#define VIRTIO_BLK_S_UNSUPP 2
#define IN_PROGRESS_STATUS  0xff

struct virtio_blk_req_header {
    uint32_t type;
    uint32_t reserved;
    uint64_t sector;
} __packed;

#endif

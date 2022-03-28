#ifndef __DISK_H__
#define __DISK_H__

#include <list.h>
#include <types.h>

struct sector_list_entry {
    list_elem_t next;
    uint64_t lba;
    uint64_t num_sectors;
};

void disk_read_sectors(uint64_t lba, uint8_t *buf, size_t num_sectors);
void disk_write_sectors(uint64_t lba, uint8_t *buf, size_t num_sectors);
void disk_write_to_sector_list(list_t *sector_list, uint8_t *buf, size_t len);
void disk_init(void);

#endif

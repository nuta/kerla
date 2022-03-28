#ifndef __FS_H__
#define __FS_H__

#include <types.h>

size_t fs_read(const char *filename, uint8_t *buf, size_t len,
               list_t *sector_list);
void fs_init(void);

#endif

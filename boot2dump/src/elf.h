#ifndef __ELF_H__
#define __ELF_H__

#include <types.h>

struct elf64_rela {
    uint64_t r_offset;
    uint64_t r_info;
    int64_t r_addend;
};

#endif

#include <disk.h>
#include <elf.h>
#include <fs.h>
#include <list.h>
#include <malloc.h>
#include <page_alloc.h>
#include <printf.h>
#include <string.h>

extern char __base[];
extern char __rela_dyn[];
extern char __rela_dyn_end[];

void main(const char *path_unterminated, size_t path_len, uint8_t *dump,
          size_t dump_len) {
    char path[256];
    memcpy(path, path_unterminated, path_len);
    path[path_len] = '\0';

    INFO("booting version %s", GIT_COMMIT);

    // Resolve relocations.
    uint64_t image_base = (uint64_t) __base;
    ASSERT(image_base != 0);
    INFO("image base: %p", image_base);
    struct elf64_rela *relocs = (struct elf64_rela *) __rela_dyn;
    for (; relocs < (struct elf64_rela *) __rela_dyn_end; relocs++) {
        uint64_t *p = (uint64_t *) (image_base + relocs->r_offset);
        *p = image_base + relocs->r_addend;
    }

    malloc_init();
    disk_init();
    fs_init();

    size_t buf_len = 64 * 1024;
    uint8_t *buf = page_alloc(ALIGN_UP(buf_len, PAGE_SIZE) / PAGE_SIZE, false);
    list_t sectors_list;
    list_init(&sectors_list);
    size_t capacity = fs_read(path, buf, buf_len, &sectors_list);

    INFO("found \"%s\": capacity = %u bytes", path, capacity);
    INFO("wriiting %u bytes into %s", capacity, path);
    disk_write_to_sector_list(&sectors_list, dump, dump_len);

    INFO("successfully wrote boot.dump, rebooting...");
    arch_reboot();
}

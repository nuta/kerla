#include "ext4.h"
#include <disk.h>
#include <fs.h>
#include <list.h>
#include <malloc.h>
#include <page_alloc.h>
#include <printf.h>
#include <string.h>

static uint64_t locate_linux_partition(void) {
    uint8_t buf[SECTOR_SIZE];

    // Load the GPT header.
    disk_read_sectors(1, (uint8_t *) buf, 1);
    struct gpt_header *gpt_header = (struct gpt_header *) buf;
    if (strncmp((char *) &gpt_header->signature, "EFI PART", 8)) {
        PANIC("gpt: GPT partition table not found");
    }

    // Look for a linux partition.
    struct gpt_entry *partitions = (struct gpt_entry *) buf;
    disk_read_sectors(gpt_header->partition_table_lba, (uint8_t *) buf, 1);
    for (uint32_t i = 0; i < 4; i++) {
        struct gpt_entry *p = &partitions[i];
        uint64_t num_sectors = p->last_lba - p->first_lba;
        uint8_t *guid = p->type_guid;
        TRACE(
            "gpt: partition[%d]: type=%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x, lba=%d (%d MiB)",
            i, guid[0], guid[1], guid[2], guid[3], guid[4], guid[5], guid[6],
            guid[7], guid[8], guid[9], guid[10], guid[11], guid[12], guid[13],
            guid[14], guid[15], p->first_lba,
            num_sectors * SECTOR_SIZE / 1024 / 1024);

        if (!memcmp(guid, GPT_LINUX_FILESYSTEM_GUID, 16)) {
            TRACE("gpt: found a linux partition at LBA %llu", p->first_lba);
            return p->first_lba;
        }
    }

    PANIC("gpt: linux partition not found");
}

static uint64_t part_lba;
static size_t bytes_per_block;
static size_t sectors_per_block;
static size_t groups_count;
static size_t inodes_per_group;
static size_t bytes_per_inode;

// A `bytes_per_block`-sized temporary buffer. Be careful when you use this:
// this buffer is shared among some functions!
static uint8_t *block_buf;

static void ext4_read_block(uint64_t block, uint8_t *buf, list_t *block_list) {
    uint64_t lba = part_lba + block * sectors_per_block;
    disk_read_sectors(lba, (uint8_t *) buf, sectors_per_block);
    if (block_list) {
        struct sector_list_entry *e = malloc(sizeof(*e));
        e->lba = lba;
        e->num_sectors = sectors_per_block;
        list_push_back(block_list, &e->next);
    }
}

static void ext4_read_group_desc_for_inode(uint64_t inode,
                                           struct ext4_group_desc *desc) {
    DEBUG_ASSERT(inode >= 2);

    size_t group_desc_offset = bytes_per_block > 1024 ? 1 : 2;
    size_t group_index = (inode - 1) / inodes_per_group;

    // Read block-group descriptors.
    ext4_read_block(group_desc_offset, block_buf, NULL);

    struct ext4_group_desc *descs = (struct ext4_group_desc *) block_buf;
    ASSERT(sizeof(struct ext4_group_desc) * group_index < bytes_per_block);
    memcpy(desc, &descs[group_index], sizeof(*desc));
}

static void ext4_read_inode(uint64_t inode_no, struct ext4_inode *inode) {
    DEBUG_ASSERT(inode_no >= 2);

    struct ext4_group_desc desc;
    ext4_read_group_desc_for_inode(inode_no, &desc);

    size_t index_in_group = (inode_no - 1) % inodes_per_group;
    size_t inodes_per_block = bytes_per_block / bytes_per_inode;
    uint64_t block_offset = index_in_group / inodes_per_block;
    size_t index_in_block = index_in_group % inodes_per_block;

    ext4_read_block(desc.inode_table + block_offset, block_buf, NULL);

    struct ext4_inode *inode_in_buf =
        (struct ext4_inode *) (&block_buf[index_in_block * bytes_per_inode]);
    memcpy(inode, inode_in_buf, sizeof(*inode));
}

static size_t ext4_read_inode_extent(struct ext4_extent *e, uint8_t **buf,
                                     size_t *len, list_t *read_blocks_list) {
    uint64_t block = ((uint64_t) e->block_start_hi << 32) + e->block_start_lo;
    uint64_t remaining = e->blocks_count;
    size_t read_len = 0;
    while (*len >= bytes_per_block && remaining > 0) {
        ext4_read_block(block, *buf, read_blocks_list);
        block += 1;
        *buf += bytes_per_block;
        *len -= bytes_per_block;
        read_len += bytes_per_block;
    }

    return read_len;
}

static size_t ext4_read_inode_data(uint64_t inode_no, uint8_t *buf, size_t len,
                                   list_t *read_blocks_list) {
    struct ext4_inode inode;
    ext4_read_inode(inode_no, &inode);

    size_t read_len = 0;
    if ((inode.flags & EXT4_INODE_FLAG_EXTENTS) != 0) {
        ASSERT(inode.extent_header.depth == 0
               && "ext4_extent_index is not yet supported");
        if (inode.extent_header.depth == 0) {
            for (uint16_t i = 0; i < inode.extent_header.entries_count; i++) {
                struct ext4_extent *e = &inode.extent_header.entries[i];
                read_len +=
                    ext4_read_inode_extent(e, &buf, &len, read_blocks_list);
            }
        }
    }

    return read_len;
}

static uint64_t ext4_get_inode_by_filename(const char *filename) {
    size_t dirents_len =
        ALIGN_UP(sizeof(struct ext4_dirent) * NUM_ROOT_DIRENTS_MAX, PAGE_SIZE);
    uint8_t *buf = page_alloc(dirents_len / PAGE_SIZE, false);

    size_t read_len = ext4_read_inode_data(INODE_ROOT_DIR, (uint8_t *) buf,
                                           dirents_len, NULL);

    size_t off = 0;
    while (off < read_len) {
        struct ext4_dirent *e = (struct ext4_dirent *) &buf[off];
        DEBUG_ASSERT(e->entry_len > 0);

        char name[256];
        memcpy(name, e->name, e->name_len);
        name[e->name_len] = 0;

        TRACE("root dir: \"%s\", type=%d, inode=%d", name, e->type, e->inode);

        if (!strcmp(name, filename)) {
            return e->inode;
        }

        off += e->entry_len;
    }

    PANIC("\"%s\" not found in the root directory", filename);
}

size_t fs_read(const char *filename, uint8_t *buf, size_t len,
               list_t *sector_list) {
    uint64_t inode = ext4_get_inode_by_filename(filename);
    return ext4_read_inode_data(inode, buf, len, sector_list);
}

void fs_init(void) {
    part_lba = locate_linux_partition();

    // Load the superblock.
    uint8_t buf[SECTOR_SIZE];
    struct ext4_superblock *sb = (struct ext4_superblock *) buf;
    disk_read_sectors(part_lba + SUPERBLOCK_SECTOR_BASE, (uint8_t *) buf, 1);
    if (sb->magic != EXT4_MAGIC) {
        PANIC("ext4: invalid magic (actual=0x%x, expected=0x%x)", sb->magic,
              EXT4_MAGIC);
    }

    bytes_per_block = 1024 << sb->log2_block_size;
    sectors_per_block = bytes_per_block / SECTOR_SIZE;
    groups_count = 1 + (sb->blocks_count - 1) / sb->blocks_per_group;
    inodes_per_group = sb->inodes_per_group;
    bytes_per_inode = sb->major_revision_level < 1 ? 128 : sb->bytes_per_inode;

    INFO("ext4: found a ext4 partition");
    TRACE("ext4: version = %d.%d", sb->major_revision_level,
          sb->minor_revision_level);
    TRACE("ext4: groups_count = %d", groups_count);
    TRACE("ext4: bytes_per_block = %d", bytes_per_block);
    TRACE("ext4: sectors_per_block = %d", sectors_per_block);
    TRACE("ext4: bytes_per_inode = %d", bytes_per_inode);

    DEBUG_ASSERT(IS_ALIGNED(bytes_per_block, SECTOR_SIZE));

    block_buf =
        page_alloc(ALIGN_UP(bytes_per_block, PAGE_SIZE) / PAGE_SIZE, false);
}

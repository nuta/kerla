#ifndef __EXT4_H__
#define __EXT4_H__

#include <types.h>

/// A GPT partition table header.
struct gpt_header {
    uint8_t signature[8];
    uint32_t revision;
    uint32_t header_size;
    uint32_t header_crc32;
    uint32_t reserved;
    uint64_t current_lba;
    uint64_t backup_lba;
    uint64_t first_lba;
    uint64_t last_lba;
    uint8_t disk_guid[16];
    uint64_t partition_table_lba;
    uint32_t partitions_count;
    uint32_t partition_entry_size;
    uint32_t partition_table_crc32;
} __packed;

#define GPT_LINUX_FILESYSTEM_GUID                                              \
    "\xaf\x3d\xc6\x0f\x83\x84\x72\x47\x8e\x79\x3d\x69\xd8\x47\x7d\xe4"

/// A GPT partition table entry.
struct gpt_entry {
    uint8_t type_guid[16];
    uint8_t unique_guid[16];
    uint64_t first_lba;
    uint64_t last_lba;
    uint64_t flags;
    uint8_t name[72];
} __packed;

#define SUPERBLOCK_SECTOR_BASE (1024 / 512)
#define EXT4_MAGIC             0xef53
#define INODE_ROOT_DIR         2
#define NUM_ROOT_DIRENTS_MAX   128

struct ext4_superblock {
    uint32_t inodes_count;
    uint32_t blocks_count;
    uint32_t reserved_blocks_count;
    uint32_t free_blocks_count;
    uint32_t free_inodes_count;
    uint32_t blocks_per_superblock;
    uint32_t log2_block_size;
    uint32_t log2_fragment_size;
    uint32_t blocks_per_group;
    uint32_t fragments_per_group;
    uint32_t inodes_per_group;
    uint32_t last_mount_time;
    uint32_t last_written_time;
    uint16_t mount_count;
    uint16_t max_mount_count;
    uint16_t magic;
    uint16_t state;
    uint16_t error_handling;
    uint16_t minor_revision_level;
    uint32_t last_fsck_time;
    uint32_t fsck_interval;
    uint32_t os_id;
    uint32_t major_revision_level;
    uint16_t reserved_blocks_uid;
    uint16_t reserved_blocks_gid;
    uint32_t first_non_reserved_inode;
    uint16_t bytes_per_inode;
} __packed;

struct ext4_group_desc {
    uint32_t block_bitmap_block;
    uint32_t inode_bitmap_block;
    uint32_t inode_table;
    uint16_t free_blocks_count;
    uint16_t free_inodes_count;
    uint16_t used_dirs_count;
    uint16_t padding;
    uint32_t reserved[3];
} __packed;

struct ext4_extent {
    uint32_t blocks_base;
    uint16_t blocks_count;
    uint16_t block_start_hi;
    uint32_t block_start_lo;
} __packed;

struct ext4_extent_header {
    uint16_t magic;
    uint16_t entries_count;
    uint16_t max;
    uint16_t depth;
    uint32_t generation;
    // Only valid if depth == 0.
    struct ext4_extent entries[];
} __packed;

struct ext4_extent_index {
    uint32_t block;
    uint32_t leaf_lo;
    uint16_t leaf_hi;
    uint16_t unused;
} __packed;

#define EXT4_INODE_FLAG_EXTENTS 0x80000

struct ext4_inode {
    uint16_t mode;
    uint16_t uid;
    uint32_t size_lo;
    uint32_t atime;
    uint32_t ctime;
    uint32_t mtime;
    uint32_t dtime;
    uint16_t gid;
    uint16_t hard_links_count;
    uint32_t blocks_count;
    uint32_t flags;
    uint32_t os_specific_1;
    union {
        struct {
            struct ext4_extent_header extent_header;
        };
        struct {
            uint32_t direct_blocks[12];
            uint32_t indirect_block;
            uint32_t double_indirect_block;
            uint32_t triple_indirect_block;
        };
    };
    uint32_t generation;
    uint32_t file_acl;
    uint32_t size_hi;
    uint32_t faddr;
    uint8_t os_specific_2[12];
} __packed;

struct ext4_dirent {
    uint32_t inode;
    uint16_t entry_len;
    uint8_t name_len;
    uint8_t type;
    char name[];
} __packed;

#endif

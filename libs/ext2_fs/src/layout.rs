use crate::tools::*;
use serde::{Serialize, Deserialize};
use serde_big_array::big_array;
big_array! {
    BigArray;
    64, 190
}

/// Structure of the super block
/// disk layout
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Ext2SuperBlock {
    s_inodes_count: U32le,      /* Inodes count */
    s_blocks_count: U32le,      /* Blocks count */
    s_r_blocks_count: U32le,    /* Reserved blocks count */
    s_free_blocks_count: U32le, /* Free blocks count */
    s_free_inode_count: U32le,  /* Free inodes count */
    s_first_data_block: U32le,  /* First data block */
    s_log_block_size: U32le,    /* Log Block size */
    s_log_frag_size: U32le,     /* Fragment size */
    s_blocks_per_group: U32le,  /* Blocks per group */
    s_frags_per_group: U32le,   /* Fragments per group */
    s_inodes_per_group: U32le,  /* Inodes per group */
    s_mtime: U32le,             /* Mount time */
    s_wtime: U32le,             /* Write time */
    s_mnt_count: U16le,         /* Mount count */
    s_max_mnt_count: U16le,     /* Maximal Mount count */
    s_magic: U16le,             /* Magic signature */
    s_state: U16le,             /* File system state */
    s_errors: U16le,            /* Behaviour when detecting errors */
    s_minor_rev_level: U16le,   /* minor revision level */
    s_lastcheck: U32le,         /* time of last check */
    s_checkinterval: U32le,     /* max. time between checks */
    s_creator_os: U32le,        /* OS */
    s_rev_level: U32le,         /* Revision level */
    s_def_resuid: U16le,        /* Default uid for reserved blocks */
    s_def_resgid: U16le,        /* Default gid for reserved blocks */
    s_first_ino: U32le,         /* First non-reserved inode */
    s_inode_size: U16le,        /* size of inode structure */
    s_block_group_nr: U16le,    /* block group of this superblock */
    s_feature_compat: U32le,    /* compatible feature set */
    s_feature_incompat: U32le,  /* incompatible feature set */
    s_feature_ro_compat: U32le, /* readonly-compatible feature set */
    s_uuid: [u8; 16],           /* 128-bit uuid for volume */
    s_volume_name: [u8; 16],  /* volume name */
    #[serde(with = "BigArray")]
    s_last_mounted: [u8; 64], /* directory where last mounted */
    s_algorithm_usage_bitmap: U32le,    /* For compression */
    s_prealloc_blocks: u8,      /* Nr of blocks to try to preallocate */
    s_prealloc_dir_blocks: u8,  /* Nr to preallocate for dirs */
    s_padding1: u16,
    s_journal_uuid: [u8; 16],   /* uuid of journal superblock */
    s_journal_inum: u32,        /* inode number of journal file */
    s_journal_dev: u32,         /* device number of journal file */
    s_last_orphan: U32le,       /* start of list of inodes to delete */
    s_hash_seed: [u32; 4],      /* HTREE hash seed */
    s_def_hash_version: u8,     /* Default hash version to use */
    s_reserved_char_pad: u8,
    s_reserved_word_pad: u16,
    s_default_mount_opts: U32le,
    s_first_meta_bg: U32le,     /* First metablock block group */
    #[serde(with = "BigArray")]
    s_reserved: [u32; 190]      /* Padding to the end of the block */
}
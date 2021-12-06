use core::fmt::Debug;

/// Structure of the super block
/// disk layout
#[repr(C)]
#[derive(Debug)]
pub struct Ext2SuperBlock {
    s_inodes_count: u32,      /* Inodes count */
    s_blocks_count: u32,      /* Blocks count */
    s_r_blocks_count: u32,    /* Reserved blocks count */
    s_free_blocks_count: u32, /* Free blocks count */
    s_free_inode_count: u32,  /* Free inodes count */
    s_first_data_block: u32,  /* First data block */
    s_log_block_size: u32,    /* Log Block size */
    s_log_frag_size: u32,     /* Fragment size */
    s_blocks_per_group: u32,  /* Blocks per group */
    s_frags_per_group: u32,   /* Fragments per group */
    s_inodes_per_group: u32,  /* Inodes per group */
    s_mtime: u32,             /* Mount time */
    s_wtime: u32,             /* Write time */
    s_mnt_count: u16,         /* Mount count */
    s_max_mnt_count: u16,     /* Maximal Mount count */
    s_magic: u16,             /* Magic signature */
    s_state: u16,             /* File system state */
    s_errors: u16,            /* Behaviour when detecting errors */
    s_minor_rev_level: u16,   /* minor revision level */
    s_lastcheck: u32,         /* time of last check */
    s_checkinterval: u32,     /* max. time between checks */
    s_creator_os: u32,        /* OS */
    s_rev_level: u32,         /* Revision level */
    s_def_resuid: u16,        /* Default uid for reserved blocks */
    s_def_resgid: u16,        /* Default gid for reserved blocks */
    s_first_ino: u32,         /* First non-reserved inode */
    s_inode_size: u16,        /* size of inode structure */
    s_block_group_nr: u16,    /* block group of this superblock */
    s_feature_compat: u32,    /* compatible feature set */
    s_feature_incompat: u32,  /* incompatible feature set */
    s_feature_ro_compat: u32, /* readonly-compatible feature set */
    s_uuid: [u8; 16],           /* 128-bit uuid for volume */
    s_volume_name: [u8; 16],  /* volume name */
    s_last_mounted: [u8; 64], /* directory where last mounted */
    s_algorithm_usage_bitmap: u32,    /* For compression */
    s_prealloc_blocks: u8,      /* Nr of blocks to try to preallocate */
    s_prealloc_dir_blocks: u8,  /* Nr to preallocate for dirs */
    s_padding1: u16,
    s_journal_uuid: [u8; 16],   /* uuid of journal superblock */
    s_journal_inum: u32,        /* inode number of journal file */
    s_journal_dev: u32,         /* device number of journal file */
    s_last_orphan: u32,       /* start of list of inodes to delete */
    s_hash_seed: [u32; 4],      /* HTREE hash seed */
    s_def_hash_version: u8,     /* Default hash version to use */
    s_reserved_char_pad: u8,
    s_reserved_word_pad: u16,
    s_default_mount_opts: u32,
    s_first_meta_bg: u32,     /* First metablock block group */
    s_reserved: [u32; 190]      /* Padding to the end of the block */
}

impl Ext2SuperBlock {
    pub fn init() -> Self {
        todo!()
    }
}
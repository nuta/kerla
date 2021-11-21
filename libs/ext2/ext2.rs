use crate::endian_tool::*;

/// default block size
pub const BLOCK_SIZE:u32 = 1024;

/// Structure of the super block
/// disk layout
#[derive(Debug)]
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
    s_volume_name: [char; 16],  /* volume name */
    s_last_mounted: [char; 64], /* directory where last mounted */
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
    s_reserved: [u32; 190]      /* Padding to the end of the block */
}

/// ext2 super block in memory
pub struct Ext2SbInfo {
    // TODO
}

impl Ext2SuperBlock {

    /// generate super block by binary
    /// this a bad ways to implement. just test to read super block in disk
    pub fn by_binary(data: &mut &[u8]) -> Option<Ext2SuperBlock> {
        if data.len() < 1024 {
            return None;
        }
        let (mut rest, s_inodes_count) = read_func::read_u32le(data);
        let (mut rest, s_blocks_count) = read_func::read_u32le(&mut rest);
        let (mut rest, s_r_blocks_count) = read_func::read_u32le(&mut rest);
        let (mut rest, s_free_blocks_count) = read_func::read_u32le(&mut rest);
        let (mut rest, s_free_inode_count) = read_func::read_u32le(&mut rest);
        let (mut rest, s_first_data_block) = read_func::read_u32le(&mut rest);
        let (mut rest, s_log_block_size) = read_func::read_u32le(&mut rest);
        let (mut rest, s_log_frag_size) = read_func::read_u32le(&mut rest);
        let (mut rest, s_blocks_per_group) = read_func::read_u32le(&mut rest);
        let (mut rest, s_frags_per_group) = read_func::read_u32le(&mut rest);
        let (mut rest, s_inodes_per_group) = read_func::read_u32le(&mut rest);
        let (mut rest, s_mtime) = read_func::read_u32le(&mut rest);
        let (mut rest, s_wtime) = read_func::read_u32le(&mut rest);
        let (mut rest, s_mnt_count) = read_func::read_u16le(&mut rest);
        let (mut rest, s_max_mnt_count) = read_func::read_u16le(&mut rest);
        let (mut rest, s_magic) = read_func::read_u16le(&mut rest);
        let (mut rest, s_state) = read_func::read_u16le(&mut rest);
        let (mut rest, s_errors) = read_func::read_u16le(&mut rest);
        let (mut rest, s_minor_rev_level) = read_func::read_u16le(&mut rest);
        let (mut rest, s_lastcheck) = read_func::read_u32le(&mut rest);
        let (mut rest, s_checkinterval) = read_func::read_u32le(&mut rest);
        let (mut rest, s_creator_os) = read_func::read_u32le(&mut rest);
        let (mut rest, s_rev_level) = read_func::read_u32le(&mut rest);
        let (mut rest, s_def_resuid) = read_func::read_u16le(&mut rest);
        let (mut rest, s_def_resgid) = read_func::read_u16le(&mut rest);
        let (mut rest, s_first_ino) = read_func::read_u32le(&mut rest);
        let (mut rest, s_inode_size) = read_func::read_u16le(&mut rest);
        let (mut rest, s_block_group_nr) = read_func::read_u16le(&mut rest);
        let (mut rest, s_feature_compat) = read_func::read_u32le(&mut rest);
        let (mut rest, s_feature_incompat) = read_func::read_u32le(&mut rest);
        let (mut rest, s_feature_ro_compat) = read_func::read_u32le(&mut rest);
        let (mut rest, s_uuid) = read_func::read_u816(&mut rest);
        let (mut rest, s_volume_name) = read_func::read_char16(&mut rest);
        let (mut rest, s_last_mounted) = read_func::read_char64(&mut rest);
        let (mut rest, s_algorithm_usage_bitmap) = read_func::read_u32le(&mut rest);
        let (mut rest, s_prealloc_blocks) = read_func::read_u8(&mut rest);
        let (mut rest, s_prealloc_dir_blocks) = read_func::read_u8(&mut rest);
        let (mut rest, s_padding1) = read_func::read_u16(&mut rest);
        let (mut rest, s_journal_uuid) = read_func::read_u816(&mut rest);
        let (mut rest, s_journal_inum) = read_func::read_u32(&mut rest);
        let (mut rest, s_journal_dev) = read_func::read_u32(&mut rest);
        let (mut rest, s_last_orphan) = read_func::read_u32le(&mut rest);
        let (mut rest, s_hash_seed) = read_func::read_u324(&mut rest);
        let (mut rest, s_def_hash_version) = read_func::read_u8(&mut rest);
        let (mut rest, s_reserved_char_pad) = read_func::read_u8(&mut rest);
        let (mut rest, s_reserved_word_pad) = read_func::read_u16(&mut rest);
        let (mut rest, s_default_mount_opts) = read_func::read_u32le(&mut rest);
        let (mut rest, s_first_meta_bg) = read_func::read_u32le(&mut rest);
        // ignore s_reserved
        let s_reserved= [0u32; 190];

        let ext2_super_block = Ext2SuperBlock {
            s_inodes_count,      /* Inodes count */
            s_blocks_count,      /* Blocks count */
            s_r_blocks_count,    /* Reserved blocks count */
            s_free_blocks_count, /* Free blocks count */
            s_free_inode_count,  /* Free inodes count */
            s_first_data_block,  /* First data block */
            s_log_block_size,    /* Log Block size */
            s_log_frag_size,     /* Fragment size */
            s_blocks_per_group,  /* Blocks per group */
            s_frags_per_group,   /* Fragments per group */
            s_inodes_per_group,  /* Inodes per group */
            s_mtime,             /* Mount time */
            s_wtime,             /* Write time */
            s_mnt_count,         /* Mount count */
            s_max_mnt_count,     /* Maximal Mount count */
            s_magic,             /* Magic signature */
            s_state,             /* File system state */
            s_errors,            /* Behaviour when detecting errors */
            s_minor_rev_level,   /* minor revision level */
            s_lastcheck,         /* time of last check */
            s_checkinterval,     /* max. time between checks */
            s_creator_os,        /* OS */
            s_rev_level,         /* Revision level */
            s_def_resuid,        /* Default uid for reserved blocks */
            s_def_resgid,        /* Default gid for reserved blocks */
            s_first_ino,         /* First non-reserved inode */
            s_inode_size,        /* size of inode structure */
            s_block_group_nr,    /* block group of this superblock */
            s_feature_compat,    /* compatible feature set */
            s_feature_incompat,  /* incompatible feature set */
            s_feature_ro_compat, /* readonly-compatible feature set */
            s_uuid,           /* 128-bit uuid for volume */
            s_volume_name,  /* volume name */
            s_last_mounted, /* directory where last mounted */
            s_algorithm_usage_bitmap,    /* For compression */
            s_prealloc_blocks,      /* Nr of blocks to try to preallocate */
            s_prealloc_dir_blocks,  /* Nr to preallocate for dirs */
            s_padding1,
            s_journal_uuid,   /* uuid of journal superblock */
            s_journal_inum,        /* inode number of journal file */
            s_journal_dev,         /* device number of journal file */
            s_last_orphan,       /* start of list of inodes to delete */
            s_hash_seed,      /* HTREE hash seed */
            s_def_hash_version,     /* Default hash version to use */
            s_reserved_char_pad,
            s_reserved_word_pad,
            s_default_mount_opts,
            s_first_meta_bg,     /* First metablock block group */
            s_reserved      /* Padding to the end of the block */
        };
        Some(ext2_super_block)
    }

    pub fn get_magic(&self) -> &U16le {
        &self.s_magic
    }

    pub fn get_inode_count(&self) -> &U32le {
        &self.s_inodes_count
    }

    pub fn get_block_count(&self) -> &U32le {
        &self.s_blocks_count
    }

    pub fn get_r_block_count(&self) -> &U32le {
        &self.s_r_blocks_count
    }

}

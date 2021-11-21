use core::mem::size_of;
use crate::ext2::Ext2SuperBlock;
use crate::SuperBlock;
use crate::error::{FileSysError, Result};
use crate::endian_tool::*;

/// read super block from disk
/// this function is flawed, it should return kernel::SuperBlock,
/// and build superBlock in memory
pub fn ext2_fill_super(data: &[u8]) -> Result<Ext2SuperBlock> {
    // 1. alloc memory to Ext2SbInfo. if fail will throw ENOMEM
    // 2. check the logical block size, should compare the default logical block size
    //    and the real logical block size. (by some information of the disk device)
    //    set the largest one to the logical block size.
    //    note: the largest one must be a power of 2 and cannot be greater than 4096
    // 3. read the super block by disk.
    //    Determine the logical block number and intra-block offset
    //    of the super block according to the block size calculated in step 2.
    //    store in the memory super block struct allocated in step 1.
    if data.len() < 2048 {
        return Err(FileSysError::EOF);
    }
    let mut super_block_data = &data[1024..2048];
    if let Some(super_block) = Ext2SuperBlock::by_binary(&mut super_block_data) {
        return Ok(super_block);
    }

    Err(FileSysError::ENOMEM)
}

/// get super block
pub fn get_sb_block(data: &[u8]) -> i64 {
    todo!()
}

use crate::layout::Ext2SuperBlock;
use crate::error::{FileSysError, Result};
use postcard::from_bytes;

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
        return Err(FileSysError::Eof);
    }
    let super_block_data = &data[1024..2048];
    let sb: Ext2SuperBlock = from_bytes(super_block_data).unwrap();
    Ok(sb)
}

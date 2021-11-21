use core::any::Any;
use crate::error::Result;

/// Block device trait, it should implement read/write from block
/// The trait should user or kernel implement and mount ext2
/// ext2 will exec it to do block buffer manager
pub trait Disk : Send + Sync + Any {
    // read block by block_id
    fn read_block(&mut self, block_id: usize, buf: &mut [u8]) -> Result<usize>;
    // write block by block_id
    fn write_block(&mut self, block_id: usize, buf: &[u8]) -> Result<usize>;
}
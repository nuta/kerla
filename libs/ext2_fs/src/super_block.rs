use crate::layout::Ext2SuperBlock;
use crate::error::{FileSysError, Result};

/// read super block by binary data
pub fn read_super_block<'a>(data: &[u8]) -> Result<&'a Ext2SuperBlock> {
    let type_size = core::mem::size_of::<Ext2SuperBlock>();
    if data.len() < type_size {
        return Err(FileSysError::Eof);
    }
    let pointer = &data[0] as *const _ as usize;
    unsafe {
        let sb = &*(pointer as *const Ext2SuperBlock);
        Ok(sb)
    }
    
}

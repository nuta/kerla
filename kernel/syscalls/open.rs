use crate::fs::{
    inode::INode,
    opened_file::{OpenFlags, OpenedFile},
    path::Path,
    stat::FileMode,
};
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use alloc::sync::Arc;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_open(&mut self, path: &Path, flags: OpenFlags) -> Result<isize> {
        let file = current_process()
            .root_fs
            .lock()
            .lookup_file(path.as_str())?;

        let fd = current_process()
            .opened_files
            .lock()
            .open(INode::FileLike(file))?;

        Ok(fd.as_usize() as isize)
    }
}

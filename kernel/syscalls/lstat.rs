use crate::fs::{inode::INode, path::Path};
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_lstat(&mut self, path: &Path, buf: UserVAddr) -> Result<isize> {
        let inode = current_process().root_fs.lock().lookup(path.as_str())?;
        let stat = match inode {
            INode::FileLike(file) => file.stat()?,
            INode::Symlink(file) => file.stat()?,
            INode::Directory(dir) => dir.stat()?,
        };

        buf.write(&stat)?;
        Ok(0)
    }
}

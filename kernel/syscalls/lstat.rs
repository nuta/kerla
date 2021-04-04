use crate::fs::path::Path;
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_lstat(&mut self, path: &Path, buf: UserVAddr) -> Result<isize> {
        let stat = current_process()
            .root_fs
            .lock()
            .lookup_no_symlink_follow(path.as_str())?
            .stat()?;
        buf.write(&stat)?;
        Ok(0)
    }
}

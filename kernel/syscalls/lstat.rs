use crate::fs::path::Path;
use crate::result::Result;
use crate::{process::current_process, syscalls::SyscallHandler};
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_lstat(&mut self, path: &Path, buf: UserVAddr) -> Result<isize> {
        let stat = current_process()
            .root_fs()
            .lock()
            .lookup_no_symlink_follow(path)?
            .stat()?;
        buf.write(&stat)?;
        Ok(0)
    }
}

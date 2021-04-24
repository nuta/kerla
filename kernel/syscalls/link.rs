use crate::fs::path::Path;
use crate::result::Result;
use crate::syscalls::{AtFlags, CwdOrFd, SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_link(&mut self, src: &Path, dst: &Path) -> Result<isize> {
        self.sys_linkat(CwdOrFd::AtCwd, src, CwdOrFd::AtCwd, dst, AtFlags::empty())
    }
}

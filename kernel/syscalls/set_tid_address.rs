use crate::syscalls::SyscallHandler;
use crate::{arch::UserVAddr, result::Result};

impl<'a> SyscallHandler<'a> {
    pub fn sys_set_tid_address(&mut self, _uaddr: UserVAddr) -> Result<isize> {
        /* TODO: */
        Ok(0)
    }
}

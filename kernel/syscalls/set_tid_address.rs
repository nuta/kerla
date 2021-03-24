use crate::syscalls::SyscallDispatcher;
use crate::{arch::UserVAddr, result::Result};

impl SyscallDispatcher {
    pub fn sys_set_tid_address(&mut self, _uaddr: UserVAddr) -> Result<isize> {
        /* TODO: */
        Ok(0)
    }
}

use crate::result::Result;
use crate::syscalls::SyscallHandler;
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_set_tid_address(&mut self, _uaddr: UserVAddr) -> Result<isize> {
        /* TODO: */
        Ok(0)
    }
}

use crate::arch::UserVAddr;
use crate::ctypes::*;
use crate::result::Result;
use crate::syscalls::SyscallDispatcher;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_rt_sigaction(
        &mut self,
        _signum: c_int,
        _act: UserVAddr,
        _oldact: UserVAddr,
    ) -> Result<isize> {
        // TODO:
        Ok(0)
    }
}

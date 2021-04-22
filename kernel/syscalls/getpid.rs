use crate::result::Result;
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_getpid(&mut self) -> Result<isize> {
        Ok(current_process().pid.as_i32() as isize)
    }
}

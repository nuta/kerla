use crate::{process::current_process, result::Result, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_gettid(&mut self) -> Result<isize> {
        Ok(current_process().tid().as_i32() as isize)
    }
}

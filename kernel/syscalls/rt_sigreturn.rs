use crate::syscalls::SyscallHandler;
use crate::{prelude::*, process::current_process};

impl<'a> SyscallHandler<'a> {
    pub fn sys_rt_sigreturn(&mut self) -> Result<isize> {
        current_process().restore_signaled_user_stack(self.frame);
        Err(Errno::EINTR.into())
    }
}

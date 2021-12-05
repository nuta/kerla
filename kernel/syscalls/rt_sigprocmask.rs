use crate::prelude::*;
use crate::process::current_process;

use crate::syscalls::SyscallHandler;
use kerla_runtime::address::UserVAddr;

impl SyscallHandler<'_> {
    pub fn sys_rt_sigprocmask(
        &mut self,
        how: usize,
        set: Option<UserVAddr>,
        oldset: Option<UserVAddr>,
        length: usize,
    ) -> Result<isize> {
        current_process().rt_sigprocmask(how, set, oldset, length)
    }
}

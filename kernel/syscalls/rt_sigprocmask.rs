use crate::prelude::*;
use crate::process::current_process;

use crate::process::signal::SignalMask;
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
        let how = match how {
            0 => SignalMask::Block,
            1 => SignalMask::Unblock,
            2 => SignalMask::Set,
            _ => return Err(Errno::EINVAL.into()),
        };

        if let Err(_) = current_process().set_signal_mask(how, set, oldset, length) {
            return Err(Errno::EFAULT.into());
        }

        Ok(0)
    }
}

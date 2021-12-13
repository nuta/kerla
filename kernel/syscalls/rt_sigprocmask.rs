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
        if length != 8 {
            debug_warn!("sys_rt_sigprocmask length argument is not equal 8");
        }

        let how = match how {
            0 => SignalMask::Block,
            1 => SignalMask::Unblock,
            2 => SignalMask::Set,
            _ => return Err(Errno::EINVAL.into()),
        };

        current_process().set_signal_mask(how, set, oldset, length)?;

        Ok(0)
    }
}

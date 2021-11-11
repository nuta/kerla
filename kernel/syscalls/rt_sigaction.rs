use crate::ctypes::*;
use crate::prelude::*;
use crate::process::current_process;
use crate::process::signal::{SigAction, DEFAULT_ACTIONS, SIG_DFL, SIG_IGN};
use crate::syscalls::SyscallHandler;
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_rt_sigaction(
        &mut self,
        signum: c_int,
        act: usize,
        _oldact: Option<UserVAddr>,
    ) -> Result<isize> {
        if let Some(act) = UserVAddr::new(act) {
            let handler = act.read::<usize>()?;
            let new_action = match handler {
                SIG_IGN => SigAction::Ignore,
                SIG_DFL => match DEFAULT_ACTIONS.get(signum as usize) {
                    Some(default_action) => *default_action,
                    None => return Err(Errno::EINVAL.into()),
                },
                _ => SigAction::Handler {
                    handler: UserVAddr::new(handler).ok_or_else(|| Error::new(Errno::EFAULT))?,
                },
            };

            current_process()
                .signals()
                .lock()
                .set_action(signum, new_action)?;
        }

        // TODO: Support oldact
        Ok(0)
    }
}

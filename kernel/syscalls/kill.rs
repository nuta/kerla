use crate::process::{current_process, signal::Signal, PId, Process};
use crate::result::Errno;
use crate::result::Result;
use crate::syscalls::SyscallHandler;

impl<'a> SyscallHandler<'a> {
    pub fn sys_kill(&self, pid: PId, sig: Signal) -> Result<isize> {
        let pid_int = pid.as_i32();
        match pid_int {
            pid_int if pid_int > 0 => match Process::find_by_pid(pid) {
                Some(proc) => proc.send_signal(sig),
                None => return Err(Errno::ESRCH.into()),
            },
            0 => current_process().process_group().lock().signal(sig),
            -1 => {
                // TODO: check for permissions once linux capabilities is implemented
                current_process().send_signal(sig);
            }
            pid_int if pid_int < -1 => current_process().process_group().lock().signal(sig),
            _ => (),
        }

        Ok(0)
    }
}

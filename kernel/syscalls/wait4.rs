use crate::{
    arch::UserVAddr,
    process::{current_process, get_process_by_pid, switch, PId},
    result::{Errno, Error, Result},
    syscalls::SyscallDispatcher,
};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_wait4(
        &mut self,
        pid: PId,
        _status: UserVAddr,
        _options: i32,
        _rusage: UserVAddr,
    ) -> Result<isize> {
        let got_pid = if pid.as_i32() == -1 {
            switch(crate::process::ProcessState::WaitForAnyChild);
            current_process().lock().resumed_by.unwrap()
        } else if pid.as_i32() == 0 {
            todo!();
        } else {
            get_process_by_pid(pid)
                .ok_or_else(|| Error::new(Errno::ECHILD))?
                .wait_queue
                .sleep();
            pid
        };

        Ok(got_pid.as_i32() as isize)
    }
}

use super::MAX_READ_WRITE_LEN;
use crate::{
    arch::UserVAddr,
    fs::opened_file::Fd,
    process::{current_process, get_process_by_pid, switch, PId},
    result::{Errno, Error, Result},
    syscalls::SyscallDispatcher,
};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_wait4(
        &mut self,
        pid: PId,
        status: UserVAddr,
        options: i32,
        rusage: UserVAddr,
    ) -> Result<isize> {
        let got_pid = if pid.as_i32() == -1 {
            switch(crate::process::ProcessState::WaitForAnyChild);
            current_process().lock().resumed_by.unwrap()
        } else if pid.as_i32() == 0 {
            todo!();
        } else {
            get_process_by_pid(pid)
                .ok_or(Error::new(Errno::ECHILD))?
                .wait_queue
                .sleep();
            pid
        };

        Ok(got_pid.as_i32() as isize)
    }
}

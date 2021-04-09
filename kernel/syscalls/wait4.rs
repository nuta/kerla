use crate::{
    arch::UserVAddr,
    ctypes::*,
    process::{current_process, PId, ProcessState, JOIN_WAIT_QUEUE},
    result::Result,
    syscalls::SyscallDispatcher,
};

use bitflags::bitflags;

bitflags! {
    pub struct WaitOptions: c_int {
        const WNOHANG   = 1;
        const WUNTRACED = 2;
    }
}

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_wait4(
        &mut self,
        pid: PId,
        status: UserVAddr,
        options: WaitOptions,
        _rusage: UserVAddr,
    ) -> Result<isize> {
        let (got_pid, status_value) = JOIN_WAIT_QUEUE.sleep_until(|| {
            let children = current_process().children.lock();
            for child in children.iter() {
                if pid.as_i32() > 0 && child.pid != pid {
                    // Wait for the specific PID.
                    continue;
                }

                if pid.as_i32() == 0 {
                    // TODO: Wait for any children in the same process group.
                    todo!();
                }

                if let ProcessState::ExitedWith(status_value) = child.state() {
                    return Ok(Some((child.pid, status_value)));
                }
            }

            if options.contains(WaitOptions::WNOHANG) {
                return Ok(Some((PId::new(0), 0)));
            }

            Ok(None)
        })?;

        // Evict joined or unused processs objects.
        current_process()
            .children
            .lock()
            .retain(|p| p.pid != got_pid && p.state() != ProcessState::Execved);

        status.write::<c_int>(&status_value)?;
        Ok(got_pid.as_i32() as isize)
    }
}

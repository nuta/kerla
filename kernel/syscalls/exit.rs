use crate::{ctypes::*, process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_exit(&mut self, status: c_int) -> ! {
        current_process().exit(status);
    }
}

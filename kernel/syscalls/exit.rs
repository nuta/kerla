use crate::{ctypes::*, process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_exit(&mut self, status: c_int) -> ! {
        current_process().exit(status);
    }
}

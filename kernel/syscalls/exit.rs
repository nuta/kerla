use crate::{ctypes::*, process::Process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_exit(&mut self, status: c_int) -> ! {
        Process::exit(status);
    }
}

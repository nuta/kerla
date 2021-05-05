use crate::{
    ctypes::*,
    process::{current_process, Process},
    syscalls::SyscallHandler,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_exit(&mut self, status: c_int) -> ! {
        Process::exit(current_process(), status);
    }
}

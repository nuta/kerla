use crate::{
    process::{current_process, Process},
    result::Result,
    syscalls::SyscallHandler,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_fork(&mut self) -> Result<isize> {
        Process::fork(current_process(), self.frame).map(|child| child.pid().as_i32() as isize)
    }
}

use crate::{process::current_process_arc, result::Result};
use crate::{process::fork::fork, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_fork(&mut self) -> Result<isize> {
        fork(current_process_arc(), self.frame).map(|child| child.lock().pid().as_i32() as isize)
    }
}

use crate::result::Result;
use crate::{
    process::{current_process, fork},
    syscalls::SyscallDispatcher,
};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_fork(&mut self) -> Result<isize> {
        fork(current_process(), self.frame).map(|child| child.pid.as_i32() as isize)
    }
}

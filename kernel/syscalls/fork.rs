use super::MAX_READ_WRITE_LEN;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{
    process::{current_process, fork},
    syscalls::SyscallDispatcher,
};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_fork(&mut self) -> Result<isize> {
        fork(current_process(), self.frame).map(|child| child.pid.as_i32() as isize)
    }
}

use crate::syscalls::SyscallDispatcher;
use crate::{
    arch::UserVAddr,
    fs::opened_file::Fd,
    result::{Errno, Error, Result},
};

impl SyscallDispatcher {
    pub fn sys_ioctl(&mut self, fd: Fd, cmd: usize, arg: usize) -> Result<isize> {
        // TODO:
        Ok(0)
    }
}

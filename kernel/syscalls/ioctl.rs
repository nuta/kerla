use crate::fs::opened_file::Fd;
use crate::result::Result;
use crate::syscalls::SyscallHandler;

impl<'a> SyscallHandler<'a> {
    pub fn sys_ioctl(&mut self, _fd: Fd, _cmd: usize, _arg: usize) -> Result<isize> {
        // TODO:
        Ok(0)
    }
}

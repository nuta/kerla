use crate::result::Result;
use crate::syscalls::SyscallHandler;
use crate::{fs::opened_file::Fd, process::current_process};

impl<'a> SyscallHandler<'a> {
    pub fn sys_ioctl(&mut self, fd: Fd, cmd: usize, arg: usize) -> Result<isize> {
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        let ret = opened_file.lock().ioctl(cmd, arg);
        ret
    }
}

use crate::fs::opened_file::Fd;
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_fstat(&mut self, fd: Fd, buf: UserVAddr) -> Result<isize> {
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        let stat = opened_file.lock().path().inode.stat()?;
        buf.write(&stat)?;
        Ok(0)
    }
}

use crate::fs::opened_file::Fd;
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_fstat(&mut self, fd: Fd, buf: UserVAddr) -> Result<isize> {
        let stat = current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .path()
            .inode
            .stat()?;
        buf.write(&stat)?;
        Ok(0)
    }
}

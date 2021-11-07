use crate::fs::opened_file::Fd;
use crate::process::current_process;
use crate::result::Result;
use crate::syscalls::SyscallHandler;

impl<'a> SyscallHandler<'a> {
    pub fn sys_fsync(&mut self, fd: Fd) -> Result<isize> {
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        opened_file.fsync()?;
        Ok(0)
    }
}

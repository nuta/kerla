use crate::fs::opened_file::Fd;
use crate::process::current_process;
use crate::result::Result;
use crate::syscalls::SyscallDispatcher;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_fsync(&mut self, fd: Fd) -> Result<isize> {
        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .fsync()?;
        Ok(0)
    }
}

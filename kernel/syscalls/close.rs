use super::MAX_READ_WRITE_LEN;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_close(&mut self, fd: Fd) -> Result<isize> {
        current_process().opened_files.lock().close(fd);
        Ok(0)
    }
}

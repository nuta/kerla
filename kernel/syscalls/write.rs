use super::MAX_READ_WRITE_LEN;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result, user_buffer::UserBuffer};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_write(&mut self, fd: Fd, uaddr: UserVAddr, len: usize) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);

        let written_len = current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .write(UserBuffer::from_uaddr(uaddr, len))?;

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(written_len as isize)
    }
}

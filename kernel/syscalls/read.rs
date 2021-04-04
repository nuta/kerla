use super::MAX_READ_WRITE_LEN;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result, user_buffer::UserBufferMut};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_read(&mut self, fd: Fd, uaddr: UserVAddr, len: usize) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);

        let current = current_process().opened_files.lock();
        let mut open_file = current.get(fd)?.lock();
        let read_len = open_file.read(UserBufferMut::from_uaddr(uaddr, len))?;

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(read_len as isize)
    }
}

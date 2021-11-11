use super::MAX_READ_WRITE_LEN;
use crate::{fs::opened_file::Fd, prelude::*, user_buffer::UserBufferMut};
use crate::{process::current_process, syscalls::SyscallHandler};
use core::cmp::min;
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_read(&mut self, fd: Fd, uaddr: UserVAddr, len: usize) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);

        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        trace!(
            "[{}:{}] read(file={:?}, len={})",
            current_process().pid().as_i32(),
            current_process().cmdline().argv0(),
            opened_file.inode(),
            len
        );

        let read_len = opened_file.read(UserBufferMut::from_uaddr(uaddr, len))?;

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(read_len as isize)
    }
}

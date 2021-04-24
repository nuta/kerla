use super::MAX_READ_WRITE_LEN;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{
    net::{socket::*, SendToFlags},
    user_buffer::UserBuffer,
};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_sendto(
        &mut self,
        fd: Fd,
        uaddr: UserVAddr,
        len: usize,
        _flags: SendToFlags,
        dst_addr: UserVAddr,
        addr_len: usize,
    ) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);
        let sockaddr = parse_sockaddr(dst_addr, addr_len)?;

        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .sendto(UserBuffer::from_uaddr(uaddr, len), sockaddr)?;

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(len as isize)
    }
}

use super::MAX_READ_WRITE_LEN;
use crate::{fs::opened_file::Fd, result::Result};
use crate::{
    net::{socket::*, SendToFlags},
    user_buffer::UserBuffer,
};
use crate::{process::current_process, syscalls::SyscallHandler};
use core::cmp::min;
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_sendto(
        &mut self,
        fd: Fd,
        uaddr: UserVAddr,
        len: usize,
        _flags: SendToFlags,
        dst_addr: Option<UserVAddr>,
        addr_len: usize,
    ) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);
        let sockaddr = match dst_addr {
            Some(dst_addr) => Some(read_sockaddr(dst_addr, addr_len)?),
            None => None,
        };

        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        let sent_len = opened_file.sendto(UserBuffer::from_uaddr(uaddr, len), sockaddr)?;

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(sent_len as isize)
    }
}

use super::MAX_READ_WRITE_LEN;
use crate::{fs::opened_file::Fd, result::Result};
use crate::{net::socket::write_sockaddr, net::RecvFromFlags, user_buffer::UserBufferMut};
use crate::{process::current_process, syscalls::SyscallHandler};
use core::cmp::min;
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_recvfrom(
        &mut self,
        fd: Fd,
        uaddr: UserVAddr,
        len: usize,
        flags: RecvFromFlags,
        src_addr: Option<UserVAddr>,
        addr_len: Option<UserVAddr>,
    ) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);

        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        let (read_len, sockaddr) =
            opened_file.recvfrom(UserBufferMut::from_uaddr(uaddr, len), flags)?;

        write_sockaddr(&sockaddr, src_addr, addr_len)?;

        // MAX_READ_WRITE_LEN limit guarantees len is in the range of isize.
        Ok(read_len as isize)
    }
}

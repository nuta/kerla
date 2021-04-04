use super::{write_endpoint_as_sockaddr, MAX_READ_WRITE_LEN};
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{net::RecvFromFlags, user_buffer::UserBufferMut};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_recvfrom(
        &mut self,
        fd: Fd,
        uaddr: UserVAddr,
        len: usize,
        flags: RecvFromFlags,
        src_addr: UserVAddr,
        addr_len: UserVAddr,
    ) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);

        let (read_len, endpoint) = current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .recvfrom(UserBufferMut::from_uaddr(uaddr, len), flags)?;

        write_endpoint_as_sockaddr(&endpoint, src_addr, addr_len)?;

        // MAX_READ_WRITE_LEN limit guarantees len is in the range of isize.
        Ok(read_len as isize)
    }
}

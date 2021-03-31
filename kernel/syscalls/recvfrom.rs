use super::{write_endpoint_as_sockaddr, SockAddrIn, MAX_READ_WRITE_LEN};
use crate::net::RecvFromFlags;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
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

        let mut buf = vec![0; len]; // TODO: deny too long len
        let (read_len, endpoint) = current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .recvfrom(buf.as_mut_slice(), flags)?;

        info!("read_len={} {}", read_len, buf[..read_len].len());
        uaddr.write_bytes(&buf[..read_len])?;
        write_endpoint_as_sockaddr(&endpoint, src_addr, addr_len)?;

        // MAX_READ_WRITE_LEN limit guarantees len is in the range of isize.
        Ok(read_len as isize)
    }
}

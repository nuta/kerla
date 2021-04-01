use super::{parse_sockaddr, MAX_READ_WRITE_LEN};
use crate::net::{Endpoint, SendToFlags};
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
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
        let endpoint: Endpoint = parse_sockaddr(dst_addr, addr_len)?.into();

        let mut wrr = vec![0; 16];
        dst_addr.read_bytes(&mut wrr).unwrap();
        info!("dst = {:02x?}", wrr);

        let mut buf = vec![0; len]; // TODO: deny too long len
        uaddr.read_bytes(&mut buf)?;
        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .sendto(buf.as_slice(), endpoint)?;

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(buf.len() as isize)
    }
}

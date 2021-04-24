use crate::fs::opened_file::Fd;
use crate::{arch::UserVAddr, net::socket::parse_sockaddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_connect(&mut self, fd: Fd, addr: UserVAddr, addr_len: usize) -> Result<isize> {
        let sockaddr = parse_sockaddr(addr, addr_len)?;
        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .connect(sockaddr)?;

        Ok(0)
    }
}

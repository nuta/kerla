use crate::fs::opened_file::Fd;
use crate::{arch::UserVAddr, result::Result};
use crate::{net::socket::parse_sockaddr, process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_bind(&mut self, fd: Fd, addr: UserVAddr, addr_len: usize) -> Result<isize> {
        let sockaddr = parse_sockaddr(addr, addr_len)?;
        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .bind(sockaddr)?;

        Ok(0)
    }
}

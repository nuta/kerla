use crate::fs::opened_file::Fd;
use crate::{net::socket::read_sockaddr, result::Result};
use crate::{process::current_process, syscalls::SyscallHandler};
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_connect(&mut self, fd: Fd, addr: UserVAddr, addr_len: usize) -> Result<isize> {
        let sockaddr = read_sockaddr(addr, addr_len)?;
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        opened_file.connect(sockaddr)?;
        Ok(0)
    }
}

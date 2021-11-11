use crate::fs::opened_file::Fd;
use crate::result::Result;
use crate::{net::socket::read_sockaddr, process::current_process, syscalls::SyscallHandler};
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_bind(&mut self, fd: Fd, addr: UserVAddr, addr_len: usize) -> Result<isize> {
        let sockaddr = read_sockaddr(addr, addr_len)?;
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        opened_file.bind(sockaddr)?;
        Ok(0)
    }
}

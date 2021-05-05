use crate::fs::opened_file::Fd;
use crate::{arch::UserVAddr, net::socket::read_sockaddr, result::Result};
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_connect(&mut self, fd: Fd, addr: UserVAddr, addr_len: usize) -> Result<isize> {
        let sockaddr = read_sockaddr(addr, addr_len)?;
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        opened_file.lock().connect(sockaddr)?;
        Ok(0)
    }
}

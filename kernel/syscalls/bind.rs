use crate::{arch::UserVAddr, result::Result};
use crate::{fs::opened_file::Fd, net::Endpoint};
use crate::{process::current_process, syscalls::SyscallDispatcher};

use super::parse_sockaddr;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_bind(&mut self, fd: Fd, addr: UserVAddr, addr_len: usize) -> Result<isize> {
        let endpoint: Endpoint = parse_sockaddr(addr, addr_len)?.into();
        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .bind(endpoint)?;

        Ok(0)
    }
}

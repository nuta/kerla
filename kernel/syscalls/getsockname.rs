use crate::{fs::opened_file::Fd, result::Result};
use crate::{net::socket::write_sockaddr, process::current_process, syscalls::SyscallHandler};
use kerla_runtime::address::UserVAddr;

impl<'a> SyscallHandler<'a> {
    pub fn sys_getsockname(
        &mut self,
        fd: Fd,
        sockaddr: UserVAddr,
        socklen: UserVAddr,
    ) -> Result<isize> {
        let endpoint = current_process()
            .opened_files()
            .lock()
            .get(fd)?
            .getsockname()?;

        write_sockaddr(&endpoint, Some(sockaddr), Some(socklen))?;
        Ok(0)
    }
}

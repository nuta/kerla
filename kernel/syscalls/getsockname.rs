use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{
    net::socket::write_endpoint_as_sockaddr, process::current_process, syscalls::SyscallHandler,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_getsockname(
        &mut self,
        fd: Fd,
        sockaddr: UserVAddr,
        socklen: UserVAddr,
    ) -> Result<isize> {
        let endpoint = current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .getsockname()?;

        write_endpoint_as_sockaddr(&endpoint, Some(sockaddr), Some(socklen))?;
        Ok(0)
    }
}

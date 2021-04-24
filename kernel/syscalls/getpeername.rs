use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{
    net::socket::write_endpoint_as_sockaddr, process::current_process, syscalls::SyscallDispatcher,
};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_getpeername(
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
            .getpeername()?;

        write_endpoint_as_sockaddr(&endpoint, sockaddr, socklen)?;
        Ok(0)
    }
}

use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

use super::write_endpoint_as_sockaddr;

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

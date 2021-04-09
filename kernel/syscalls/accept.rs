use crate::{
    arch::UserVAddr,
    fs::opened_file::{Fd, OpenOptions},
    result::Result,
};
use crate::{process::current_process, syscalls::SyscallDispatcher};

use super::write_endpoint_as_sockaddr;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_accept(&mut self, fd: Fd, sockaddr: UserVAddr, socklen: UserVAddr) -> Result<isize> {
        let (sock, endpoint) = current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .accept()?;

        let options = OpenOptions {
            nonblock: false,
            close_on_exec: false,
        };
        let fd = current_process()
            .opened_files
            .lock()
            .open(sock.into(), options)?;
        write_endpoint_as_sockaddr(&endpoint, sockaddr, socklen)?;
        Ok(fd.as_usize() as isize)
    }
}

use crate::{
    arch::UserVAddr,
    fs::opened_file::{Fd, OpenOptions, PathComponent},
    net::socket::write_endpoint_as_sockaddr,
    prelude::*,
    process::current_process,
    syscalls::SyscallHandler,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_accept(
        &mut self,
        fd: Fd,
        sockaddr: Option<UserVAddr>,
        socklen: Option<UserVAddr>,
    ) -> Result<isize> {
        let (sock, accepted_sockaddr) = current_process()
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
            .open(PathComponent::new_anonymous(sock.into()), options)?;
        write_endpoint_as_sockaddr(&accepted_sockaddr, sockaddr, socklen)?;
        Ok(fd.as_usize() as isize)
    }
}

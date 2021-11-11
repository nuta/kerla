use kerla_runtime::address::UserVAddr;

use crate::{
    fs::opened_file::{Fd, OpenOptions, PathComponent},
    net::socket::write_sockaddr,
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
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        let (sock, accepted_sockaddr) = opened_file.accept()?;

        let options = OpenOptions {
            nonblock: false,
            close_on_exec: false,
        };
        let fd = current_process()
            .opened_files()
            .lock()
            .open(PathComponent::new_anonymous(sock.into()), options)?;
        write_sockaddr(&accepted_sockaddr, sockaddr, socklen)?;
        Ok(fd.as_usize() as isize)
    }
}

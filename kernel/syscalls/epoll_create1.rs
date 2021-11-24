use bitflags::bitflags;

use crate::{
    ctypes::c_int,
    epoll::EPoll,
    fs::{
        inode::INode,
        opened_file::{OpenOptions, PathComponent},
    },
    prelude::*,
    process::current_process,
    syscalls::SyscallHandler,
};

bitflags! {
    pub struct EPollCreateFlags: c_int {
        const EPOLL_CLOEXEC = 0o2000000;
    }
}

impl<'a> SyscallHandler<'a> {
    pub fn sys_epoll_create1(&mut self, flags: EPollCreateFlags) -> Result<isize> {
        let mut options = OpenOptions::empty();
        if flags.contains(EPollCreateFlags::EPOLL_CLOEXEC) {
            options.close_on_exec = true;
        }

        let epoll = EPoll::new();
        let epfd = current_process()
            .opened_files()
            .lock()
            .open(PathComponent::new_anonymous(INode::EPoll(epoll)), options)?;

        Ok(epfd.as_int() as isize)
    }
}

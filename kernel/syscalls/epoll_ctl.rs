use kerla_runtime::address::UserVAddr;

use crate::{
    ctypes::c_int,
    fs::{inode::PollStatus, opened_file::Fd},
    prelude::*,
    process::current_process,
    syscalls::SyscallHandler,
};

const EPOLL_CTL_ADD: c_int = 1;
const EPOLL_CTL_DEL: c_int = 2;

impl<'a> SyscallHandler<'a> {
    pub fn sys_epoll_ctl(
        &mut self,
        epfd: Fd,
        op: c_int,
        fd: Fd,
        event: Option<UserVAddr>,
    ) -> Result<isize> {
        let epoll_file = current_process().get_opened_file_by_fd(epfd)?;
        let epoll = epoll_file.as_epoll()?;
        match op {
            EPOLL_CTL_ADD => {
                let target = current_process().get_opened_file_by_fd(fd)?;

                // Read `events` from `struct epoll_event`.
                let events_raw: u32 = event.ok_or_else(|| Error::new(Errno::EFAULT))?.read()?;
                let events = PollStatus::from_bits_truncate(events_raw);
                epoll.add(&target, fd, events)?;
            }
            EPOLL_CTL_DEL => {
                let target = current_process().get_opened_file_by_fd(fd)?;
                epoll.del(&target, fd)?;
            }
            _ => {
                return Err(Error::new(Errno::EINVAL));
            }
        }

        Ok(0)
    }
}

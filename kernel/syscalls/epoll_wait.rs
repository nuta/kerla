use core::mem::size_of;

use kerla_runtime::address::UserVAddr;

use crate::{
    ctypes::c_int,
    epoll::{EPollData, EPollEvent},
    fs::opened_file::Fd,
    prelude::*,
    process::current_process,
    syscalls::SyscallHandler,
    user_buffer::UserBufWriter,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_epoll_wait(
        &mut self,
        epfd: Fd,
        events: UserVAddr,
        maxevents: c_int,
        timeout: c_int,
    ) -> Result<isize> {
        let epoll_file = current_process().get_opened_file_by_fd(epfd)?;
        let epoll = epoll_file.as_epoll()?;
        let mut n = 0;
        let mut events_writer =
            UserBufWriter::from_uaddr(events, size_of::<EPollEvent>() * (maxevents as usize));
        epoll.wait(timeout, |fd, events| {
            events_writer.write(EPollEvent {
                events: events.bits() as u32,
                data: EPollData { fd: fd.as_int() },
            })?;
            n += 1;
            Ok(())
        })?;

        Ok(n)
    }
}

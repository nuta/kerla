use crate::ctypes::c_int;
use crate::fs::opened_file::Fd;
use crate::net::ShutdownHow;
use crate::result::Result;
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_shutdown(&mut self, fd: Fd, _how: c_int) -> Result<isize> {
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        opened_file.shutdown(ShutdownHow::RdWr /* FIXME: */)?;
        Ok(0)
    }
}

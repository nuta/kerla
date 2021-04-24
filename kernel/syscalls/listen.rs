use crate::ctypes::*;
use crate::{fs::opened_file::Fd, result::Result};
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_listen(&mut self, fd: Fd, backlog: c_int) -> Result<isize> {
        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .listen(backlog)?;

        Ok(0)
    }
}

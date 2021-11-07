use crate::ctypes::*;
use crate::{fs::opened_file::Fd, result::Result};
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_listen(&mut self, fd: Fd, backlog: c_int) -> Result<isize> {
        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        opened_file.listen(backlog)?;
        Ok(0)
    }
}

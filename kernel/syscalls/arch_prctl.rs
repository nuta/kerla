use crate::{
    arch::{arch_prctl, UserVAddr},
    fs::opened_file::Fd,
    result::{Errno, Error, Result},
};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl SyscallDispatcher {
    pub fn sys_arch_prctl(&mut self, code: i32, uaddr: UserVAddr) -> Result<isize> {
        arch_prctl(current_process(), code, uaddr)?;
        Ok(0)
    }
}

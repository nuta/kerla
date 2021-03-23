use crate::{
    arch::{arch_prctl, UserVAddr},
    fs::opened_file::Fd,
    result::{Errno, Error, Result},
};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl SyscallDispatcher {
    pub fn sys_exit(&mut self, status: i32) -> ! {
        todo!()
    }
}

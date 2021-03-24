
use crate::{syscalls::SyscallDispatcher};

impl SyscallDispatcher {
    pub fn sys_exit(&mut self, _status: i32) -> ! {
        todo!()
    }
}

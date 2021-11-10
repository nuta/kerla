use crate::{process::current_process, result::Result, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    /// returns the caller's thread ID (TID).  
    /// In a single-threaded process, the thread ID is equal to the process ID (PID)
    pub fn sys_gettid(&mut self) -> Result<isize> {
        Ok(current_process().pid().as_i32() as isize)
    }
}

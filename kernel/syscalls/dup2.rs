use crate::fs::opened_file::{Fd, OpenOptions};
use crate::prelude::*;
use crate::process::current_process;
use crate::syscalls::SyscallHandler;

impl<'a> SyscallHandler<'a> {
    pub fn sys_dup2(&mut self, old: Fd, new: Fd) -> Result<isize> {
        let current = current_process();
        let mut opened_files = current.opened_files().lock();
        opened_files.dup2(old, new, OpenOptions::new(false, false))?;
        Ok(new.as_int() as isize)
    }
}

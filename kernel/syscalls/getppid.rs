use crate::{
    prelude::*,
    process::process_group::PgId,
    process::{current_process, process_group::ProcessGroup, PId, Process},
    result::Result,
    syscalls::SyscallHandler,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_getppid(&mut self) -> Result<isize> {
        Ok(current_process().ppid().as_i32() as isize)
    }
}

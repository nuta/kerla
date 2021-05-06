use crate::{process::current_process, syscalls::SyscallHandler};
use crate::{process::PId, result::Result};

impl<'a> SyscallHandler<'a> {
    pub fn sys_getpgid(&mut self, pid: PId) -> Result<isize> {
        let pgid = if pid.as_i32() == 0 {
            current_process().process_group().lock().pgid()
        } else {
            todo!()
        };

        Ok(pgid.as_i32() as isize)
    }
}

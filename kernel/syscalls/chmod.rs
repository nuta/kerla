use crate::fs::{path::Path, stat::FileMode};
use crate::result::Result;
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_chmod(&mut self, path: &Path, mode: FileMode) -> Result<isize> {
        current_process()
            .root_fs()
            .lock()
            .lookup(path)?
            .chmod(mode)?;
        Ok(0)
    }
}

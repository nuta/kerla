use crate::fs::{path::Path, stat::FileMode};
use crate::result::Result;
use crate::syscalls::SyscallHandler;

impl<'a> SyscallHandler<'a> {
    pub fn sys_chmod(&mut self, _path: &Path, _mode: FileMode) -> Result<isize> {
        // TODO:
        Ok(0)
    }
}

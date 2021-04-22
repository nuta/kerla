use crate::fs::{path::Path, stat::FileMode};
use crate::result::Result;
use crate::syscalls::SyscallDispatcher;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_chmod(&mut self, _path: &Path, _mode: FileMode) -> Result<isize> {
        // TODO:
        Ok(0)
    }
}

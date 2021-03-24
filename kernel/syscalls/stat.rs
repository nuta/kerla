use crate::fs::path::Path;
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl SyscallDispatcher {
    pub fn sys_stat(&mut self, path: &Path, buf: UserVAddr) -> Result<isize> {
        let file = current_process()
            .root_fs
            .lock()
            .lookup_file(path.as_str())?;
        let stat = file.stat()?;
        buf.write(&stat)?;
        Ok(0)
    }
}

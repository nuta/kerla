use crate::fs::path::Path;
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_utimes(&mut self, path: &Path, _times: UserVAddr) -> Result<isize> {
        // TODO: Currently we don't modify the file metadata: Return ENOENT if
        //       the file exists for touch(1).
        current_process().root_fs.lock().lookup_file(path)?;
        Ok(0)
    }
}

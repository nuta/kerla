use crate::fs::path::Path;
use crate::result::Result;
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_chdir(&mut self, path: &Path) -> Result<isize> {
        // Check if the directory exists.
        current_process().root_fs.lock().lookup_dir(path.as_str())?;
        current_process().lock().chdir(path);
        Ok(0)
    }
}

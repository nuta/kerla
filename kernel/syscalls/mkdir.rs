use crate::fs::{path::Path, stat::FileMode};
use crate::result::{Errno, Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_mkdir(&mut self, path: &Path, mode: FileMode) -> Result<isize> {
        let (parent_dir, name) = match path.parent_and_basename() {
            Some((parent_dir, name)) => (parent_dir, name),
            None => {
                // Tried to create the root directory.
                return Err(Errno::EEXIST.into());
            }
        };

        let created_dir = current_process()
            .root_fs
            .lock()
            .lookup_dir(parent_dir.as_str())?
            .create_dir(name)?;

        let fd = current_process().opened_files.lock().open(created_dir)?;
        Ok(fd.as_usize() as isize)
    }
}

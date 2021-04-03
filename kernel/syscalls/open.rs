use crate::fs::{opened_file::OpenFlags, path::Path};
use crate::result::Result;
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_open(&mut self, path: &Path, flags: OpenFlags) -> Result<isize> {
        let inode = if flags.contains(OpenFlags::O_DIRECTORY) {
            current_process()
                .root_fs
                .lock()
                .lookup_dir(path.as_str())?
                .into()
        } else {
            current_process()
                .root_fs
                .lock()
                .lookup_file(path.as_str())?
                .into()
        };

        let fd = current_process().opened_files.lock().open(inode)?;
        Ok(fd.as_usize() as isize)
    }
}

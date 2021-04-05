use crate::fs::{inode::INode, opened_file::OpenFlags, path::Path};
use crate::result::{Errno, Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

fn open_file(path: &Path, flags: OpenFlags) -> Result<INode> {
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

    Ok(inode)
}

fn create_file(path: &Path, flags: OpenFlags) -> Result<INode> {
    if flags.contains(OpenFlags::O_DIRECTORY) {
        // A directory should be created through mkdir(2).
        return Err(Errno::EINVAL.into());
    }

    let (parent_dir, name) = match path.parent_and_basename() {
        Some((parent_dir, name)) => (parent_dir, name),
        None => {
            // Tried to create the root directory.
            return Err(Errno::EEXIST.into());
        }
    };

    current_process()
        .root_fs
        .lock()
        .lookup_dir(parent_dir.as_str())?
        .create_file(name)
}

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_open(&mut self, path: &Path, flags: OpenFlags) -> Result<isize> {
        let inode = if flags.contains(OpenFlags::O_CREAT) {
            create_file(path, flags)?
        } else {
            open_file(path, flags)?
        };

        let fd = current_process().opened_files.lock().open(inode)?;
        Ok(fd.as_usize() as isize)
    }
}

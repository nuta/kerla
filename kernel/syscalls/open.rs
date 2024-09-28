use super::CwdOrFd;
use crate::fs::stat::{O_RDWR, O_WRONLY};
use crate::fs::{inode::INode, opened_file::OpenFlags, path::Path, stat::FileMode};
use crate::prelude::*;
use crate::{process::current_process, syscalls::SyscallHandler};

fn create_file(path: &Path, flags: OpenFlags, mode: FileMode) -> Result<INode> {
    if flags.contains(OpenFlags::O_DIRECTORY) {
        // A directory should be created through mkdir(2).
        return Err(Errno::EINVAL.into());
    }

    let (parent_dir, name) = path
        .parent_and_basename()
        .ok_or_else::<Error, _>(|| Errno::EEXIST.into())?;

    current_process()
        .root_fs()
        .lock()
        .lookup_dir(parent_dir)?
        .create_file(name, mode)
}

impl<'a> SyscallHandler<'a> {
    pub fn sys_open(&mut self, path: &Path, flags: OpenFlags, mode: FileMode) -> Result<isize> {
        let current = current_process();
        trace!(
            "[{}:{}] open(\"{}\")",
            current.pid().as_i32(),
            current.cmdline().argv0(),
            path.as_str()
        );

        if flags.contains(OpenFlags::O_CREAT) {
            match create_file(path, flags, mode) {
                Ok(_) => {}
                Err(err) if flags.contains(OpenFlags::O_EXCL) && err.errno() == Errno::EEXIST => {}
                Err(err) => {
                    return Err(err);
                }
            }
        }

        let root_fs = current.root_fs().lock();
        let mut opened_files = current.opened_files().lock();

        let path_comp = root_fs.lookup_path_at(&opened_files, &CwdOrFd::AtCwd, path, true)?;
        if flags.contains(OpenFlags::O_DIRECTORY) && !path_comp.inode.is_dir() {
            return Err(Error::new(Errno::ENOTDIR));
        }

        let access_mode = mode.access_mode();
        if path_comp.inode.is_dir() && (access_mode == O_WRONLY || access_mode == O_RDWR) {
            return Err(Error::new(Errno::EISDIR));
        }

        let fd = opened_files.open(path_comp, flags.into())?;
        Ok(fd.as_usize() as isize)
    }
}

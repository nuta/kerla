use crate::fs::path::Path;
use crate::result::Result;
use crate::{
    process::current_process,
    syscalls::{AtFlags, CwdOrFd, SyscallHandler},
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_linkat(
        &mut self,
        src_dir: CwdOrFd,
        src_path: &Path,
        dst_dir: CwdOrFd,
        dst_path: &Path,
        flags: AtFlags,
    ) -> Result<isize> {
        let current = current_process();
        let root_fs = current.root_fs().lock();
        let opened_files = current.opened_files().lock();
        let src = root_fs.lookup_path_at(
            &opened_files,
            &src_dir,
            src_path,
            flags.contains(AtFlags::AT_SYMLINK_FOLLOW),
        )?;
        let (parent_dir, dst_name) =
            root_fs.lookup_parent_path_at(&opened_files, &dst_dir, dst_path, true)?;
        parent_dir.inode.as_dir()?.link(dst_name, &src.inode)?;
        Ok(0)
    }
}

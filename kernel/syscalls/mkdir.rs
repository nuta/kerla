use crate::fs::{path::Path, stat::FileMode};
use crate::prelude::*;
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_mkdir(&mut self, path: &Path, mode: FileMode) -> Result<isize> {
        let (parent_dir, name) = path
            .parent_and_basename()
            .ok_or_else::<Error, _>(|| Errno::EEXIST.into())?;

        current_process()
            .root_fs()
            .lock()
            .lookup_dir(parent_dir)?
            .create_dir(name, mode)?;

        Ok(0)
    }
}

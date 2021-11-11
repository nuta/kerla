use kerla_runtime::address::UserVAddr;

use crate::fs::path::Path;
use crate::prelude::*;
use crate::{process::current_process, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_utimes(&mut self, path: &Path, _times: Option<UserVAddr>) -> Result<isize> {
        // TODO: Currently we don't modify the file metadata: Return ENOENT if
        //       the file exists for touch(1).
        current_process().root_fs().lock().lookup_file(path)?;
        Ok(0)
    }
}

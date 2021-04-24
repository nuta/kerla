use crate::syscalls::SyscallHandler;
use crate::{
    arch::UserVAddr,
    result::{Errno, Result},
};
use crate::{ctypes::*, process::current_process};

use crate::user_buffer::UserBufWriter;

impl<'a> SyscallHandler<'a> {
    pub fn sys_getcwd(&mut self, buf: UserVAddr, size: c_size) -> Result<isize> {
        let cwd = current_process()
            .root_fs
            .lock()
            .cwd_path()
            .resolve_absolute_path();

        if (size as usize) < cwd.as_str().as_bytes().len() {
            return Err(Errno::ERANGE.into());
        }

        let mut writer = UserBufWriter::new(buf);
        writer.write_bytes(cwd.as_str().as_bytes())?;
        writer.write(0u8)?;
        Ok(buf.as_isize())
    }
}

use kerla_runtime::address::UserVAddr;

use crate::result::Result;
use crate::syscalls::SyscallHandler;
use crate::{
    fs::{opened_file::Fd, path::Path},
    process::current_process,
    result::Errno,
};

use crate::user_buffer::UserBufWriter;

impl<'a> SyscallHandler<'a> {
    pub fn sys_readlink(&mut self, path: &Path, buf: UserVAddr, buf_size: usize) -> Result<isize> {
        let resolved_path = if path.as_str().starts_with("/proc/self/fd/") {
            // TODO: Implement procfs
            let fd = path.as_str()["/proc/self/fd/".len()..].parse().unwrap();
            current_process()
                .opened_files()
                .lock()
                .get(Fd::new(fd))?
                .path()
                .resolve_absolute_path()
        } else {
            current_process()
                .root_fs()
                .lock()
                .lookup_no_symlink_follow(path)?
                .readlink()?
        };

        if buf_size < resolved_path.as_str().as_bytes().len() {
            return Err(Errno::ERANGE.into());
        }

        let mut writer = UserBufWriter::from_uaddr(buf, buf_size);
        writer.write_bytes(resolved_path.as_str().as_bytes())?;
        writer.write(0u8)?;
        Ok(writer.pos() as isize)
    }
}

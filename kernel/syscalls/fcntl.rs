use crate::fs::opened_file::Fd;
use crate::result::{Errno, Result};
use crate::syscalls::SyscallDispatcher;
use crate::{ctypes::*, process::current_process};

const _F_DUPFD: c_int = 0;
const _F_GETFD: c_int = 1;
const _F_SETFD: c_int = 2;
const _F_GETFL: c_int = 3;
const _F_SETFL: c_int = 4;

// Linux-specific commands.
const F_LINUX_SPECIFIC_BASE: c_int = 1024;
const F_DUPFD_CLOEXEC: c_int = F_LINUX_SPECIFIC_BASE + 6;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_fcntl(&mut self, fd: Fd, cmd: c_int, arg: usize) -> Result<isize> {
        match cmd {
            F_DUPFD_CLOEXEC => {
                let fd = current_process()
                    .opened_files
                    .lock()
                    .dup(fd, Some(arg as i32), true)?;
                Ok(fd.as_int() as isize)
            }
            _ => Err(Errno::ENOSYS.into()),
        }
    }
}

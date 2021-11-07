use crate::fs::opened_file::{Fd, OpenFlags, OpenOptions};
use crate::result::{Errno, Result};
use crate::syscalls::SyscallHandler;
use crate::{ctypes::*, process::current_process};

const _F_DUPFD: c_int = 0;
const _F_GETFD: c_int = 1;
const F_SETFD: c_int = 2;
const _F_GETFL: c_int = 3;
const F_SETFL: c_int = 4;

// Linux-specific commands.
const F_LINUX_SPECIFIC_BASE: c_int = 1024;
const F_DUPFD_CLOEXEC: c_int = F_LINUX_SPECIFIC_BASE + 6;

impl<'a> SyscallHandler<'a> {
    pub fn sys_fcntl(&mut self, fd: Fd, cmd: c_int, arg: usize) -> Result<isize> {
        let current = current_process();
        let mut opened_files = current.opened_files().lock();
        match cmd {
            F_SETFD => {
                opened_files.get(fd)?.set_cloexec(arg == 1);
                Ok(0)
            }
            F_SETFL => {
                opened_files
                    .get(fd)?
                    .set_flags(OpenFlags::from_bits_truncate(arg as i32))?;
                Ok(0)
            }
            F_DUPFD_CLOEXEC => {
                let fd = opened_files.dup(fd, Some(arg as i32), OpenOptions::new(false, true))?;
                Ok(fd.as_int() as isize)
            }
            _ => Err(Errno::ENOSYS.into()),
        }
    }
}

use crate::{
    fs::{opened_file::Fd, path::Path},
    arch::{SyscallFrame, UserVAddr},
    process::PId,
    result::{Errno, Error, Result},
};
use alloc::vec::Vec;

const SYS_READ: usize = 0;
const SYS_WRITE: usize = 1;
const SYS_STAT: usize = 4;
const SYS_BRK: usize = 12;
const SYS_IOCTL: usize = 16;
const SYS_WRITEV: usize = 20;
const SYS_FORK: usize = 57;
const SYS_EXECVE: usize = 59;
const SYS_EXIT: usize = 60;
const SYS_WAIT4: usize = 61;
const SYS_ARCH_PRCTL: usize = 158;
const SYS_SET_TID_ADDRESS: usize = 218;

const PATH_MAX: usize = 512;

pub(super) struct UserCStr {
    buf: Vec<u8>,
}

impl UserCStr {
    pub fn new(uaddr: UserVAddr, max_len: usize) -> Result<UserCStr> {
        let mut buf = Vec::with_capacity(max_len);
        buf.resize(max_len, 0);
        let copied_len = uaddr.read_cstr(buf.as_mut_slice())?;
        buf.resize(copied_len, 0);
        Ok(UserCStr { buf })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_str(&self) -> Result<&str> {
        core::str::from_utf8(&self.buf).map_err(|_| Error::new(Errno::EINVAL))
    }
}

pub struct SyscallDispatcher<'a> {
    pub frame: &'a SyscallFrame,
}

impl<'a> SyscallDispatcher<'a> {
    pub fn new(frame: &'a SyscallFrame) -> SyscallDispatcher<'a> {
        SyscallDispatcher { frame }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn dispatch(
        &mut self,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        _a5: usize,
        _a6: usize,
        n: usize,
    ) -> Result<isize> {
        match n {
            SYS_READ => self.sys_read(Fd::new(a1 as i32), UserVAddr::new(a2)?, a3),
            SYS_WRITE => self.sys_write(Fd::new(a1 as i32), UserVAddr::new(a2)?, a3),
            SYS_WRITEV => self.sys_writev(Fd::new(a1 as i32), UserVAddr::new(a2)?, a3),
            SYS_STAT => self.sys_stat(
                Path::new(UserCStr::new(UserVAddr::new(a1)?, PATH_MAX)?.as_str()?),
                UserVAddr::new(a2)?,
            ),
            SYS_ARCH_PRCTL => self.sys_arch_prctl(a1 as i32, UserVAddr::new(a2)?),
            SYS_BRK => self.sys_brk(UserVAddr::new(a1)?),
            SYS_IOCTL => self.sys_ioctl(Fd::new(a1 as i32), a2, a3),
            SYS_SET_TID_ADDRESS => self.sys_set_tid_address(UserVAddr::new(a1)?),
            SYS_EXECVE => self.sys_execve(
                Path::new(UserCStr::new(UserVAddr::new(a1)?, PATH_MAX)?.as_str()?),
                UserVAddr::new(a2)?,
                UserVAddr::new(a3)?,
            ),
            SYS_FORK => self.sys_fork(),
            SYS_WAIT4 => self.sys_wait4(
                PId::new(a1 as i32),
                UserVAddr::new(a2)?,
                a3 as i32,
                UserVAddr::new(a4)?,
            ),
            SYS_EXIT => self.sys_exit(a1 as i32),
            _ => {
                debug_warn!("unimplemented system call n={}", n);
                Err(Error::new(Errno::ENOSYS))
            }
        }
    }
}

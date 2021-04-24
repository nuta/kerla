use crate::{arch::UserVAddr, ctypes::*};
use crate::{fs::opened_file::Fd, result::Result};
use bitflags::bitflags;
use core::cmp::min;
use core::mem::size_of;
use penguin_utils::alignment::align_up;

macro_rules! bitflags_from_user {
    ($st:tt, $input:expr) => {{
        let bits = $input;
        $st::from_bits(bits).ok_or_else(|| {
            warn_once!(
                concat!("unsupported bitflags for ", stringify!($st), ": {:x}"),
                bits
            );

            crate::result::Error::new(crate::result::Errno::ENOSYS)
        })
    }};
}

pub(self) mod accept;
pub(self) mod arch_prctl;
pub(self) mod bind;
pub(self) mod brk;
pub(self) mod chdir;
pub(self) mod chmod;
pub(self) mod clock_gettime;
pub(self) mod close;
pub(self) mod connect;
pub(self) mod dispatcher;
pub(self) mod dup2;
pub(self) mod execve;
pub(self) mod exit;
pub(self) mod fcntl;
pub(self) mod fork;
pub(self) mod fstat;
pub(self) mod fsync;
pub(self) mod getcwd;
pub(self) mod getdents64;
pub(self) mod getpeername;
pub(self) mod getpid;
pub(self) mod getrandom;
pub(self) mod getsockname;
pub(self) mod ioctl;
pub(self) mod link;
pub(self) mod linkat;
pub(self) mod listen;
pub(self) mod lstat;
pub(self) mod mkdir;
pub(self) mod mmap;
pub(self) mod open;
pub(self) mod pipe;
pub(self) mod poll;
pub(self) mod read;
pub(self) mod readlink;
pub(self) mod recvfrom;
pub(self) mod rt_sigaction;
pub(self) mod select;
pub(self) mod sendto;
pub(self) mod set_tid_address;
pub(self) mod socket;
pub(self) mod stat;
pub(self) mod uname;
pub(self) mod utimes;
pub(self) mod wait4;
pub(self) mod write;
pub(self) mod writev;

pub use dispatcher::SyscallDispatcher;

pub enum CwdOrFd {
    /// `AT_FDCWD`
    AtCwd,
    Fd(Fd),
}

impl CwdOrFd {
    pub fn parse(value: c_int) -> CwdOrFd {
        match value {
            -100 => CwdOrFd::AtCwd,
            _ => CwdOrFd::Fd(Fd::new(value)),
        }
    }
}

bitflags! {
    pub struct AtFlags: c_int {
        const AT_SYMLINK_FOLLOW = 0x400;
    }
}

pub(self) const MAX_READ_WRITE_LEN: usize = core::isize::MAX as usize;
pub(self) const IOV_MAX: usize = 1024;

#[repr(C)]
pub(self) struct IoVec {
    base: UserVAddr,
    len: usize,
}

/// `struct timeval`
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct Timeval {
    tv_sec: c_time,
    tv_usec: c_suseconds,
}

impl Timeval {
    pub fn as_msecs(&self) -> usize {
        (self.tv_sec as usize) * 1000 + (self.tv_usec as usize) / 1000
    }
}

pub(self) fn parse_timeval(uaddr: Option<UserVAddr>) -> Result<Option<Timeval>> {
    match uaddr {
        Some(uaddr) => Ok(Some(uaddr.read::<Timeval>()?)),
        None => Ok(None),
    }
}

pub struct UserBufReader {
    base: UserVAddr,
    pos: usize,
}

impl UserBufReader {
    pub const fn new(base: UserVAddr) -> UserBufReader {
        UserBufReader { base, pos: 0 }
    }

    pub fn pos(&mut self) -> usize {
        self.pos
    }

    pub fn skip(&mut self, len: usize) {
        self.pos += len;
    }

    pub fn read<T: Copy>(&mut self) -> Result<T> {
        let value = self.base.add(self.pos)?.read()?;
        self.pos += size_of::<T>();
        Ok(value)
    }
}

pub struct UserBufWriter {
    base: UserVAddr,
    pos: usize,
}

impl UserBufWriter {
    pub const fn new(base: UserVAddr) -> UserBufWriter {
        UserBufWriter { base, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn skip_until_alignment(&mut self, align: usize) {
        self.pos = align_up(self.pos, align);
    }

    pub fn fill(&mut self, value: u8, len: usize) -> Result<()> {
        self.pos += self.base.add(self.pos)?.fill(value, len)?;
        Ok(())
    }

    pub fn write<T: Copy>(&mut self, value: T) -> Result<()> {
        let written_len = self.base.add(self.pos)?.write(&value)?;
        self.pos += written_len;
        Ok(())
    }

    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        let written_len = self.base.add(self.pos)?.write_bytes(buf)?;
        self.pos += written_len;
        Ok(())
    }

    pub fn write_bytes_or_zeroes(&mut self, buf: &[u8], max_copy_len: usize) -> Result<()> {
        let zeroed_after = min(buf.len(), max_copy_len);
        self.write_bytes(&buf[..zeroed_after])?;
        self.fill(0, max_copy_len - zeroed_after)?;
        Ok(())
    }
}

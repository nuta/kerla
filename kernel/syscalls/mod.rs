use crate::arch::UserVAddr;

pub(self) mod arch_prctl;
pub(self) mod brk;
pub(self) mod dispatcher;
pub(self) mod exit;
pub(self) mod ioctl;
pub(self) mod read;
pub(self) mod set_tid_address;
pub(self) mod write;
pub(self) mod writev;

pub use dispatcher::SyscallDispatcher;

pub(self) const MAX_READ_WRITE_LEN: usize = core::isize::MAX as usize;
pub(self) const IOV_MAX: usize = 1024;

#[repr(C)]
pub(self) struct IoVec {
    base: UserVAddr,
    len: usize,
}

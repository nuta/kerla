use crate::syscalls::SyscallDispatcher;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};

struct _PollFd {
    /// The target file.
    fd: Fd,
    /// Requested events.
    events: i16,
    /// Returned events.
    revents: i16,
}

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_poll(&mut self, _fds: UserVAddr, _nfds: usize, _timeout: i32) -> Result<isize> {
        // TODO:
        for _ in 0..0x300000u64 {
            unsafe {
                asm!("in al, 0x80", out("rax") _);
            }
        }

        warn!("poll: not yet implemented");
        Ok(1)
    }
}

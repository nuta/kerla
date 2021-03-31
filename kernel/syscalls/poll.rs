use super::MAX_READ_WRITE_LEN;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

struct PollFd {
    /// The target file.
    fd: Fd,
    /// Requested events.
    events: i16,
    /// Returned events.
    revents: i16,
}

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_poll(&mut self, fds: UserVAddr, nfds: usize, timeout: i32) -> Result<isize> {
        // TODO:
        for i in 0..0x300000u64 {
            unsafe {
                asm!("in al, 0x80", out("rax") _);
            }
        }

        warn!("poll: not yet implemented");
        Ok(1)
    }
}

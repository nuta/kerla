use super::{IoVec, IOV_MAX, MAX_READ_WRITE_LEN};
use crate::prelude::*;
use crate::{fs::opened_file::Fd, user_buffer::UserBuffer};
use crate::{process::current_process, syscalls::SyscallHandler};
use core::cmp::min;
use kerla_runtime::address::UserVAddr;

use core::mem::size_of;

impl<'a> SyscallHandler<'a> {
    pub fn sys_writev(&mut self, fd: Fd, iov_base: UserVAddr, iov_count: usize) -> Result<isize> {
        let iov_count = min(iov_count, IOV_MAX);

        let opened_file = current_process().get_opened_file_by_fd(fd)?;
        let mut total_len: usize = 0;
        for i in 0..iov_count {
            // Read an entry from the userspace.
            let mut iov: IoVec = iov_base.add(i * size_of::<IoVec>()).read()?;

            // Handle the case when total_len exceed the limit.
            match total_len.checked_add(iov.len) {
                Some(len) if len > MAX_READ_WRITE_LEN => {
                    iov.len = MAX_READ_WRITE_LEN - total_len;
                }
                None => {
                    iov.len = MAX_READ_WRITE_LEN - total_len;
                }
                _ => {}
            }

            if iov.len == 0 {
                continue;
            }

            total_len += opened_file.write(UserBuffer::from_uaddr(iov.base, iov.len))?;
        }

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(total_len as isize)
    }
}

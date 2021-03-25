use super::{IoVec, IOV_MAX, MAX_READ_WRITE_LEN};
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

use core::mem::size_of;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_writev(&mut self, fd: Fd, iov_base: UserVAddr, iov_count: usize) -> Result<isize> {
        let iov_count = min(iov_count, IOV_MAX);

        let current = current_process().opened_files.lock();
        let open_file = current.get(fd)?.lock();
        let file = open_file.as_file()?;

        let mut total_len: usize = 0;
        for i in 0..iov_count {
            // Read an entry from the userspace.
            let mut iov: IoVec = iov_base.add(i * size_of::<IoVec>())?.read()?;

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

            let mut buf = vec![0; iov.len]; // TODO: deny too long len
            iov.base.read_bytes(&mut buf)?;
            total_len += file.write(open_file.pos(), buf.as_slice())?;
        }

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(total_len as isize)
    }
}

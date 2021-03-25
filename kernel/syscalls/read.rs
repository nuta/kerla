use super::MAX_READ_WRITE_LEN;
use crate::{arch::UserVAddr, fs::opened_file::Fd, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::cmp::min;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_read(&mut self, fd: Fd, uaddr: UserVAddr, len: usize) -> Result<isize> {
        let len = min(len, MAX_READ_WRITE_LEN);

        let current = current_process().opened_files.lock();
        let open_file = current.get(fd)?;
        let file = open_file.as_file()?;

        let mut buf = vec![0; len]; // TODO: deny too long len
        let len = file.read(open_file.pos(), buf.as_mut_slice())?;

        uaddr.write_bytes(&buf)?;

        // MAX_READ_WRITE_LEN limit guarantees total_len is in the range of isize.
        Ok(len as isize)
    }
}

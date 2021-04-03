use crate::fs::opened_file::Fd;
use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use core::mem::size_of;
use penguin_utils::alignment::align_up;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_getdents64(&mut self, fd: Fd, dirp: UserVAddr, len: usize) -> Result<isize> {
        let mut offset = 0;
        let opened_files = current_process().opened_files.lock();
        let mut dir = opened_files.get(fd)?.lock();
        loop {
            let entry = match dir.readdir()? {
                Some(entry) => entry,
                None => break,
            };

            let reclen = align_up(
                size_of::<u64>() * 2 + size_of::<u16>() + 1 + entry.name.len() + 1,
                size_of::<u64>(),
            );

            if offset + reclen > len {
                break;
            }

            // Fill a `struct linux_dirent64`.
            let head_offset = offset;
            // d_ino
            offset += dirp.add(offset)?.write::<u64>(&entry.inode_no.as_u64())?;
            // d_off
            offset += dirp.add(offset)?.write::<u64>(&(dir.pos() as u64))?;
            // d_reclen
            offset += dirp.add(offset)?.write::<u16>(&(reclen as u16))?;
            // d_type
            offset += dirp.add(offset)?.write::<u8>(&(entry.file_type as u8))?;
            // d_name
            offset += dirp.add(offset)?.write_bytes(entry.name.as_bytes())?;
            // d_name (null character)
            dirp.add(offset)?.write::<u8>(&0)?;
            offset = head_offset + reclen;
        }

        Ok(offset as isize)
    }
}

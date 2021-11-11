use crate::fs::opened_file::Fd;
use crate::result::Result;
use crate::{process::current_process, syscalls::SyscallHandler};
use core::mem::size_of;
use kerla_runtime::address::UserVAddr;
use kerla_utils::alignment::align_up;

use crate::user_buffer::UserBufWriter;

impl<'a> SyscallHandler<'a> {
    pub fn sys_getdents64(&mut self, fd: Fd, dirp: UserVAddr, len: usize) -> Result<isize> {
        let current = current_process();
        let opened_files = current.opened_files().lock();
        let dir = opened_files.get(fd)?;
        let mut writer = UserBufWriter::from_uaddr(dirp, len);
        while let Some(entry) = dir.readdir()? {
            let alignment = size_of::<u64>();
            let reclen = align_up(
                size_of::<u64>() * 2 + size_of::<u16>() + 1 + entry.name.len() + 1,
                alignment,
            );

            if writer.pos() + reclen > len {
                break;
            }

            // Fill a `struct linux_dirent64`.
            // d_ino
            writer.write::<u64>(entry.inode_no.as_u64())?;
            // d_off
            writer.write::<u64>(dir.pos() as u64)?;
            // d_reclen
            writer.write::<u16>(reclen as u16)?;
            // d_type
            writer.write::<u8>(entry.file_type as u8)?;
            // d_name
            writer.write_bytes(entry.name.as_bytes())?;
            // d_name (null character)
            writer.write::<u8>(0)?;

            writer.skip_until_alignment(alignment)?;
        }

        Ok(writer.pos() as isize)
    }
}

use core::mem::size_of;

use alloc::sync::Arc;
use kerla_runtime::address::UserVAddr;

use crate::{
    ctypes::*,
    fs::{
        inode::{FileLike, INode},
        opened_file::{OpenOptions, PathComponent},
    },
    pipe::Pipe,
    result::Result,
};
use crate::{process::current_process, syscalls::SyscallHandler};

use crate::user_buffer::UserBufWriter;

impl<'a> SyscallHandler<'a> {
    pub fn sys_pipe(&mut self, fds: UserVAddr) -> Result<isize> {
        let options = OpenOptions::empty();

        let pipe = Pipe::new();
        let read_fd = current_process().opened_files().lock().open(
            PathComponent::new_anonymous(INode::FileLike(pipe.read_end() as Arc<dyn FileLike>)),
            options,
        )?;

        let write_fd = current_process().opened_files().lock().open(
            PathComponent::new_anonymous(INode::FileLike(pipe.write_end() as Arc<dyn FileLike>)),
            options,
        )?;

        let mut fds_writer = UserBufWriter::from_uaddr(fds, 2 * size_of::<c_int>());
        fds_writer.write::<c_int>(read_fd.as_int())?;
        fds_writer.write::<c_int>(write_fd.as_int())?;
        Ok(0)
    }
}

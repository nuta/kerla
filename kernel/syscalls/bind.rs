use crate::{arch::UserVAddr, result::Result};
use crate::{
    fs::{
        inode::INode,
        opened_file::{Fd, OpenFlags, OpenedFile},
        path::Path,
        stat::FileMode,
    },
    net::Endpoint,
};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use alloc::sync::Arc;

use super::{parse_sockaddr, socklen_t};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_bind(&mut self, fd: Fd, addr: UserVAddr, addr_len: usize) -> Result<isize> {
        let endpoint: Endpoint = parse_sockaddr(addr, addr_len)?.into();
        current_process()
            .opened_files
            .lock()
            .get(fd)?
            .lock()
            .bind(endpoint)?;

        Ok(0)
    }
}

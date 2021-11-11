use crate::{ctypes::c_int, fs::opened_file::Fd, prelude::*};
use kerla_runtime::address::UserVAddr;

use super::SyscallHandler;

impl<'a> SyscallHandler<'a> {
    pub fn sys_getsockopt(
        &mut self,
        _fd: Fd,
        _level: c_int,
        _optname: c_int,
        _optval: Option<UserVAddr>,
        _optlen: Option<UserVAddr>,
    ) -> Result<isize> {
        // TODO:
        debug_warn!("getsockopt is not implemented");
        Ok(0)
    }
}

use crate::syscalls::SyscallDispatcher;
use crate::{arch::UserVAddr, result::Result};

use super::UserBufWriter;

/// The maximum length of a field in `struct utsname` including the trailing
/// null character.
const UTS_FIELD_LEN: usize = 65;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_uname(&mut self, buf: UserVAddr) -> Result<isize> {
        let mut writer = UserBufWriter::new(buf);
        // sysname
        writer.write_bytes_or_zeroes(b"Linux", UTS_FIELD_LEN)?;
        // nodename
        writer.write_bytes_or_zeroes(b"", UTS_FIELD_LEN)?;
        // release
        writer.write_bytes_or_zeroes(b"", UTS_FIELD_LEN)?;
        // version
        writer.write_bytes_or_zeroes(b"penguin", UTS_FIELD_LEN)?;
        // machine
        writer.write_bytes_or_zeroes(b"", UTS_FIELD_LEN)?;
        // domainname
        writer.write_bytes_or_zeroes(b"", UTS_FIELD_LEN)?;
        Ok(0)
    }
}

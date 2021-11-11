use crate::result::Result;
use crate::syscalls::SyscallHandler;
use kerla_runtime::address::UserVAddr;

use crate::user_buffer::UserBufWriter;

/// The maximum length of a field in `struct utsname` including the trailing
/// null character.
const UTS_FIELD_LEN: usize = 65;

impl<'a> SyscallHandler<'a> {
    pub fn sys_uname(&mut self, buf: UserVAddr) -> Result<isize> {
        let mut writer = UserBufWriter::from_uaddr(buf, 6 * UTS_FIELD_LEN);
        // sysname
        writer.write_bytes_or_zeroes(b"Linux", UTS_FIELD_LEN)?;
        // nodename
        writer.write_bytes_or_zeroes(b"", UTS_FIELD_LEN)?;
        // release
        // We use a hard-coded release number instead of using our own version
        // because glibc checks the kernel version to determine supported
        // Linux's kernel features.
        writer.write_bytes_or_zeroes(b"4.0.0", UTS_FIELD_LEN)?;
        // version
        writer.write_bytes_or_zeroes(b"Kerla", UTS_FIELD_LEN)?;
        // machine
        writer.write_bytes_or_zeroes(b"", UTS_FIELD_LEN)?;
        // domainname
        writer.write_bytes_or_zeroes(b"", UTS_FIELD_LEN)?;
        Ok(0)
    }
}

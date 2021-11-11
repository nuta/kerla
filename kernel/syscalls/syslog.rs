use kerla_runtime::address::UserVAddr;

use crate::{
    ctypes::c_int,
    logger::{KERNEL_LOG_BUF, KERNEL_LOG_BUF_SIZE},
    result::Errno,
    result::Result,
    syscalls::SyscallHandler,
    user_buffer::UserBufWriter,
};

const SYSLOG_ACTION_READ_ALL: c_int = 3;
const SYSLOG_ACTION_CONSOLE_LEVEL: c_int = 8;
const SYSLOG_ACTION_SIZE_BUFFER: c_int = 10;

impl<'a> SyscallHandler<'a> {
    pub fn sys_syslog(
        &mut self,
        type_: c_int,
        buf: Option<UserVAddr>,
        len: c_int,
    ) -> Result<isize> {
        match type_ {
            SYSLOG_ACTION_READ_ALL => {
                let buf = match buf {
                    Some(buf) => buf,
                    None => return Err(Errno::EINVAL.into()),
                };

                // Note: You can't use any printk function (including info! and
                // its friends) until you release this lock.
                let mut lock = KERNEL_LOG_BUF.lock();
                let mut writer = UserBufWriter::from_uaddr(buf, len as usize);
                while let Some(slice) = lock.pop_slice(writer.remaining_len()) {
                    writer.write_bytes(slice)?;
                }

                Ok(writer.written_len() as isize)
            }
            SYSLOG_ACTION_SIZE_BUFFER => Ok(KERNEL_LOG_BUF_SIZE as isize),
            SYSLOG_ACTION_CONSOLE_LEVEL => {
                trace!("syslog: SYSLOG_ACTION_CONSOLE_LEVEL is not yet implemented");
                Ok(0)
            }
            _ => Err(Errno::EINVAL.into()),
        }
    }
}

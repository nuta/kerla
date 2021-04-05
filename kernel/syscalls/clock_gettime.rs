use super::UserBufWriter;
use crate::syscalls::SyscallDispatcher;
use crate::{
    arch::UserVAddr,
    result::{Errno, Result},
};
use crate::{
    ctypes::{c_clockid, c_long, c_time, CLOCK_REALTIME},
    timer::read_wall_clock,
};
use core::convert::TryInto;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_clock_gettime(&mut self, clock: c_clockid, buf: UserVAddr) -> Result<isize> {
        let (tv_sec, tv_nsec) = match clock {
            CLOCK_REALTIME => {
                let now = read_wall_clock();
                (now.secs_from_epoch(), now.nanosecs_from_epoch())
            }
            _ => {
                debug_warn!("clock_gettime: unsupported clock id: {}", clock);
                return Err(Errno::ENOSYS.into());
            }
        };

        let mut writer = UserBufWriter::new(buf);
        writer.write::<c_time>(tv_sec.try_into().unwrap())?;
        writer.write::<c_long>(tv_nsec.try_into().unwrap())?;

        Ok(0)
    }
}

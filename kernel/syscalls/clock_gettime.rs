use kerla_runtime::address::UserVAddr;

use crate::result::{Errno, Result};
use crate::user_buffer::UserBufWriter;
use crate::{
    ctypes::{c_clockid, c_long, c_time, CLOCK_MONOTONIC, CLOCK_REALTIME},
    timer::read_wall_clock,
};
use crate::{syscalls::SyscallHandler, timer::read_monotonic_clock};
use core::{convert::TryInto, mem::size_of};

impl<'a> SyscallHandler<'a> {
    pub fn sys_clock_gettime(&mut self, clock: c_clockid, buf: UserVAddr) -> Result<isize> {
        let (tv_sec, tv_nsec) = match clock {
            CLOCK_REALTIME => {
                let now = read_wall_clock();
                (now.secs_from_epoch(), now.nanosecs_from_epoch())
            }
            CLOCK_MONOTONIC => {
                let now = read_monotonic_clock();
                (now.secs(), now.nanosecs())
            }
            _ => {
                debug_warn!("clock_gettime: unsupported clock id: {}", clock);
                return Err(Errno::ENOSYS.into());
            }
        };

        let mut writer = UserBufWriter::from_uaddr(buf, size_of::<c_time>() + size_of::<c_long>());
        writer.write::<c_time>(tv_sec.try_into().unwrap())?;
        writer.write::<c_long>(tv_nsec.try_into().unwrap())?;

        Ok(0)
    }
}

use crate::random::{read_insecure_random, read_secure_random};
use crate::result::Result;
use crate::syscalls::SyscallHandler;
use crate::{ctypes::c_uint, user_buffer::UserBufferMut};
use bitflags::bitflags;
use core::cmp::min;
use kerla_runtime::address::UserVAddr;

const GETRANDOM_LEN_MAX: usize = 256;

bitflags! {
    pub struct GetRandomFlags: c_uint {
        const GRND_NONBLOCK = 0x1;
        const GRND_RANDOM   = 0x2;
    }
}

impl<'a> SyscallHandler<'a> {
    pub fn sys_getrandom(
        &mut self,
        buf: UserVAddr,
        len: usize,
        flags: GetRandomFlags,
    ) -> Result<isize> {
        let len = min(len, GETRANDOM_LEN_MAX);
        let read_len = if flags.contains(GetRandomFlags::GRND_RANDOM) {
            read_secure_random(UserBufferMut::from_uaddr(buf, len))?
        } else {
            read_insecure_random(UserBufferMut::from_uaddr(buf, len))?
        };

        Ok(read_len as isize)
    }
}

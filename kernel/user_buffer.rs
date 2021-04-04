use crate::arch::UserVAddr;
use crate::result::Result;
use core::{cmp::min, mem::size_of, slice};

enum InnerMut<'a> {
    Slice(&'a mut [u8]),
    User { base: UserVAddr, len: usize },
}

pub struct UserBufferMut<'a> {
    inner: InnerMut<'a>,
    pos: usize,
}

impl<'a> UserBufferMut<'a> {
    fn from_uaddr(uaddr: UserVAddr, len: usize) -> UserBufferMut<'static> {
        UserBufferMut {
            inner: InnerMut::User { base: uaddr, len },
            pos: 0,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining_len(&self) -> usize {
        let len = match &self.inner {
            InnerMut::Slice(slice) => slice.len(),
            InnerMut::User { len, .. } => *len,
        };

        len - self.pos
    }

    pub fn write_bytes(&mut self, src: &[u8]) -> Result<usize> {
        let copy_len = min(self.remaining_len(), src.len());
        if copy_len == 0 {
            return Ok(0);
        }

        match &mut self.inner {
            InnerMut::Slice(dst) => {
                dst[self.pos..(self.pos + copy_len)].copy_from_slice(&src[..copy_len]);
            }
            InnerMut::User { base, .. } => {
                base.add(self.pos)?.write_bytes(&src[..copy_len])?;
            }
        }

        self.pos += copy_len;
        Ok(copy_len)
    }

    pub fn write<T: Copy>(&mut self, value: T) -> Result<usize> {
        let bytes =
            unsafe { slice::from_raw_parts(&value as *const T as *const u8, size_of::<T>()) };
        self.write_bytes(bytes)
    }
}

impl<'a> From<&'a mut [u8]> for UserBufferMut<'a> {
    fn from(slice: &'a mut [u8]) -> UserBufferMut<'a> {
        UserBufferMut {
            inner: InnerMut::Slice(slice),
            pos: 0,
        }
    }
}

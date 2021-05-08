//! User pointers.
use crate::arch::UserVAddr;
use crate::prelude::*;
use core::{cmp::min, mem::size_of, slice};
use penguin_utils::alignment::align_up;

/// Parses a bitflags field given from the user. Returns `Result<T>`.
macro_rules! bitflags_from_user {
    ($st:tt, $input:expr) => {{
        let bits = $input;
        $st::from_bits(bits).ok_or_else(|| {
            warn_once!(
                concat!("unsupported bitflags for ", stringify!($st), ": {:x}"),
                bits
            );

            crate::result::Error::new(crate::result::Errno::ENOSYS)
        })
    }};
}

enum Inner<'a> {
    Slice(&'a [u8]),
    User { base: UserVAddr, len: usize },
}

/// A user or kernel pointer.
pub struct UserBuffer<'a> {
    inner: Inner<'a>,
    pos: usize,
}

impl<'a> UserBuffer<'a> {
    pub fn from_uaddr(uaddr: UserVAddr, len: usize) -> UserBuffer<'static> {
        UserBuffer {
            inner: Inner::User { base: uaddr, len },
            pos: 0,
        }
    }

    pub fn remaining_len(&self) -> usize {
        let len = match &self.inner {
            Inner::Slice(slice) => slice.len(),
            Inner::User { len, .. } => *len,
        };

        len - self.pos
    }

    pub fn read_bytes(&mut self, dst: &mut [u8]) -> Result<usize> {
        let copy_len = min(self.remaining_len(), dst.len());
        if copy_len == 0 {
            return Ok(0);
        }

        match &self.inner {
            Inner::Slice(src) => {
                dst[..copy_len].copy_from_slice(&src[self.pos..(self.pos + copy_len)]);
            }
            Inner::User { base, .. } => {
                base.add(self.pos)?.read_bytes(&mut dst[..copy_len])?;
            }
        }

        self.pos += copy_len;
        Ok(copy_len)
    }
}

impl<'a> From<&'a [u8]> for UserBuffer<'a> {
    fn from(slice: &'a [u8]) -> UserBuffer<'a> {
        UserBuffer {
            inner: Inner::Slice(slice),
            pos: 0,
        }
    }
}

enum InnerMut<'a> {
    Slice(&'a mut [u8]),
    User { base: UserVAddr, len: usize },
}

pub struct UserBufferMut<'a> {
    inner: InnerMut<'a>,
    pos: usize,
}

impl<'a> UserBufferMut<'a> {
    pub fn from_uaddr(uaddr: UserVAddr, len: usize) -> UserBufferMut<'static> {
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

    pub fn _write<T: Copy>(&mut self, value: T) -> Result<usize> {
        let bytes =
            unsafe { slice::from_raw_parts(&value as *const T as *const u8, size_of::<T>()) };
        self.write_bytes(bytes)
    }

    pub fn write_with<F>(&mut self, mut f: F) -> Result<usize>
    where
        F: FnMut(&mut [u8]) -> Result<usize>,
    {
        match &mut self.inner {
            InnerMut::Slice(slice) => {
                let written_len = f(slice)?;
                self.pos += written_len;
                Ok(written_len)
            }
            InnerMut::User { base, len } => {
                let mut total_len = 0;
                let mut buf = [0; 256];
                loop {
                    let copy_len = min(buf.len(), *len - total_len);
                    let written_len = f(&mut buf.as_mut_slice()[..copy_len])?;
                    if written_len == 0 {
                        return Ok(total_len);
                    }

                    base.add(self.pos)?
                        .write_bytes(&buf.as_slice()[..copy_len])?;
                    self.pos += written_len;
                    total_len += written_len;
                }
            }
        }
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

pub struct UserBufReader {
    base: UserVAddr,
    pos: usize,
}

impl UserBufReader {
    pub const fn new(base: UserVAddr) -> UserBufReader {
        UserBufReader { base, pos: 0 }
    }

    pub fn pos(&mut self) -> usize {
        self.pos
    }

    pub fn skip(&mut self, len: usize) {
        self.pos += len;
    }

    pub fn read<T: Copy>(&mut self) -> Result<T> {
        let value = self.base.add(self.pos)?.read()?;
        self.pos += size_of::<T>();
        Ok(value)
    }
}

pub struct UserBufWriter {
    base: UserVAddr,
    pos: usize,
}

impl UserBufWriter {
    pub const fn new(base: UserVAddr) -> UserBufWriter {
        UserBufWriter { base, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn skip_until_alignment(&mut self, align: usize) {
        self.pos = align_up(self.pos, align);
    }

    pub fn fill(&mut self, value: u8, len: usize) -> Result<()> {
        self.pos += self.base.add(self.pos)?.fill(value, len)?;
        Ok(())
    }

    pub fn write<T: Copy>(&mut self, value: T) -> Result<()> {
        let written_len = self.base.add(self.pos)?.write(&value)?;
        self.pos += written_len;
        Ok(())
    }

    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        let written_len = self.base.add(self.pos)?.write_bytes(buf)?;
        self.pos += written_len;
        Ok(())
    }

    pub fn write_bytes_or_zeroes(&mut self, buf: &[u8], max_copy_len: usize) -> Result<()> {
        let zeroed_after = min(buf.len(), max_copy_len);
        self.write_bytes(&buf[..zeroed_after])?;
        self.fill(0, max_copy_len - zeroed_after)?;
        Ok(())
    }
}

pub(super) struct UserCStr {
    buf: Vec<u8>,
}

impl UserCStr {
    pub fn new(uaddr: UserVAddr, max_len: usize) -> Result<UserCStr> {
        let mut buf = vec![0; max_len];
        let copied_len = uaddr.read_cstr(buf.as_mut_slice())?;
        buf.resize(copied_len, 0);
        Ok(UserCStr { buf })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_str(&self) -> Result<&str> {
        core::str::from_utf8(&self.buf).map_err(|_| Error::new(Errno::EINVAL))
    }
}

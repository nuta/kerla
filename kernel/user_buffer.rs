//! User pointers.
use crate::prelude::*;
use core::{cmp::min, mem::size_of, slice};
use kerla_runtime::address::UserVAddr;
use kerla_utils::alignment::align_up;

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

#[derive(Debug, Clone)]
enum Inner<'a> {
    Slice(&'a [u8]),
    User { base: UserVAddr, len: usize },
}

/// A user or kernel pointer.
#[derive(Debug, Clone)]
pub struct UserBuffer<'a> {
    inner: Inner<'a>,
}

impl<'a> UserBuffer<'a> {
    pub fn from_uaddr(uaddr: UserVAddr, len: usize) -> UserBuffer<'static> {
        UserBuffer {
            inner: Inner::User { base: uaddr, len },
        }
    }

    pub fn len(&self) -> usize {
        match &self.inner {
            Inner::Slice(slice) => slice.len(),
            Inner::User { len, .. } => *len,
        }
    }
}

impl<'a> From<&'a [u8]> for UserBuffer<'a> {
    fn from(slice: &'a [u8]) -> UserBuffer<'a> {
        UserBuffer {
            inner: Inner::Slice(slice),
        }
    }
}

enum InnerMut<'a> {
    Slice(&'a mut [u8]),
    User { base: UserVAddr, len: usize },
}

pub struct UserBufferMut<'a> {
    inner: InnerMut<'a>,
}

impl<'a> UserBufferMut<'a> {
    pub fn from_uaddr(uaddr: UserVAddr, len: usize) -> UserBufferMut<'static> {
        UserBufferMut {
            inner: InnerMut::User { base: uaddr, len },
        }
    }

    pub fn len(&self) -> usize {
        match &self.inner {
            InnerMut::Slice(slice) => slice.len(),
            InnerMut::User { len, .. } => *len,
        }
    }
}

impl<'a> From<&'a mut [u8]> for UserBufferMut<'a> {
    fn from(slice: &'a mut [u8]) -> UserBufferMut<'a> {
        UserBufferMut {
            inner: InnerMut::Slice(slice),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserBufReader<'a> {
    buf: UserBuffer<'a>,
    pos: usize,
}

impl<'a> UserBufReader<'a> {
    pub fn buffer_len(&self) -> usize {
        self.buf.len()
    }

    pub fn remaining_len(&self) -> usize {
        self.buffer_len() - self.pos
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    fn check_remaining_len(&self, len: usize) -> Result<()> {
        debug_assert!(self.pos <= self.buffer_len());

        if len <= self.remaining_len() {
            Ok(())
        } else {
            Err(Errno::EINVAL.into())
        }
    }

    pub fn skip(&mut self, len: usize) -> Result<()> {
        self.check_remaining_len(len)?;
        self.pos += len;
        Ok(())
    }

    /// Reads a (plain old) object and advances the position.
    ///
    /// Returns `EINVAL` if the remaining buffer is too short.
    pub fn read<T: Copy>(&mut self) -> Result<T> {
        self.check_remaining_len(size_of::<T>())?;

        let value = match &self.buf.inner {
            Inner::Slice(src) => unsafe { *(src.as_ptr().add(self.pos) as *const T) },
            Inner::User { base, .. } => base.add(self.pos).read()?,
        };

        self.pos += size_of::<T>();
        Ok(value)
    }

    /// Reads at most* `dst.len()` bytes and advances the position.
    pub fn read_bytes(&mut self, dst: &mut [u8]) -> Result<usize> {
        let copy_len = min(self.remaining_len(), dst.len());
        if copy_len == 0 {
            return Ok(0);
        }

        match &self.buf.inner {
            Inner::Slice(src) => {
                dst[..copy_len].copy_from_slice(&src[self.pos..(self.pos + copy_len)]);
            }
            Inner::User { base, .. } => {
                base.add(self.pos).read_bytes(&mut dst[..copy_len])?;
            }
        }

        self.pos += copy_len;
        Ok(copy_len)
    }
}

impl<'a> From<UserBuffer<'a>> for UserBufReader<'a> {
    fn from(buf: UserBuffer<'a>) -> UserBufReader<'a> {
        UserBufReader { buf, pos: 0 }
    }
}

pub struct UserBufWriter<'a> {
    buf: UserBufferMut<'a>,
    pos: usize,
}

impl<'a> UserBufWriter<'a> {
    pub fn from_uaddr(buf: UserVAddr, len: usize) -> UserBufWriter<'a> {
        UserBufWriter::from(UserBufferMut::from_uaddr(buf, len))
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn written_len(&self) -> usize {
        self.pos()
    }

    pub fn buffer_len(&self) -> usize {
        self.buf.len()
    }

    pub fn remaining_len(&self) -> usize {
        self.buffer_len() - self.pos
    }

    #[inline]
    fn check_remaining_len(&self, len: usize) -> Result<()> {
        debug_assert!(self.pos <= self.buffer_len());

        if len <= self.remaining_len() {
            Ok(())
        } else {
            Err(Errno::EINVAL.into())
        }
    }

    pub fn skip_until_alignment(&mut self, align: usize) -> Result<()> {
        let new_pos = align_up(self.pos, align);
        self.check_remaining_len(new_pos - self.pos)?;
        self.pos = new_pos;
        Ok(())
    }

    pub fn fill(&mut self, value: u8, len: usize) -> Result<()> {
        self.check_remaining_len(len)?;

        match &mut self.buf.inner {
            InnerMut::Slice(dst) => {
                dst[self.pos..(self.pos + len)].fill(value);
            }
            InnerMut::User { base, .. } => {
                base.add(self.pos).fill(value, len)?;
            }
        }

        self.pos += len;
        Ok(())
    }

    pub fn write_bytes_or_zeroes(&mut self, buf: &[u8], max_copy_len: usize) -> Result<()> {
        let zeroed_after = min(buf.len(), max_copy_len);
        self.check_remaining_len(zeroed_after)?;

        self.write_bytes(&buf[..zeroed_after])?;
        self.fill(0, max_copy_len - zeroed_after)?;
        Ok(())
    }

    pub fn write_bytes(&mut self, src: &[u8]) -> Result<usize> {
        let copy_len = min(self.remaining_len(), src.len());
        if copy_len == 0 {
            return Ok(0);
        }

        self.check_remaining_len(copy_len)?;

        match &mut self.buf.inner {
            InnerMut::Slice(dst) => {
                dst[self.pos..(self.pos + copy_len)].copy_from_slice(&src[..copy_len]);
            }
            InnerMut::User { base, .. } => {
                base.add(self.pos).write_bytes(&src[..copy_len])?;
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

    pub fn write_with<F>(&mut self, mut f: F) -> Result<usize>
    where
        F: FnMut(&mut [u8]) -> Result<usize>,
    {
        match &mut self.buf.inner {
            InnerMut::Slice(slice) => {
                let written_len = f(&mut slice[self.pos..])?;
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

                    base.add(self.pos)
                        .write_bytes(&buf.as_slice()[..copy_len])?;
                    self.pos += written_len;
                    total_len += written_len;
                }
            }
        }
    }
}

impl<'a> From<UserBufferMut<'a>> for UserBufWriter<'a> {
    fn from(buf: UserBufferMut<'a>) -> UserBufWriter<'a> {
        UserBufWriter { buf, pos: 0 }
    }
}

impl<'a> core::fmt::Write for UserBufWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_bytes(s.as_bytes())
            .map_err(|_| core::fmt::Error)?;
        Ok(())
    }
}

/// A user-provided NULL-terminated string.
///
/// It's a copy of the string (not a reference) since the user can modify the
/// buffer anytime to cause bad things in the kernel.
pub(super) struct UserCStr {
    string: String,
}

impl UserCStr {
    pub fn new(uaddr: UserVAddr, max_len: usize) -> Result<UserCStr> {
        let mut tmp = vec![0; max_len];
        let copied_len = uaddr.read_cstr(tmp.as_mut_slice())?;
        let string = core::str::from_utf8(&tmp[..copied_len])
            .map_err(|_| Error::new(Errno::EINVAL))?
            .to_owned();
        Ok(UserCStr { string })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }

    pub fn as_str(&self) -> &str {
        &self.string
    }
}

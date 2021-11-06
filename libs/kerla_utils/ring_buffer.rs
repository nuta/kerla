use core::{cmp::min, mem::MaybeUninit, ops::Range, slice};

pub struct RingBuffer<T, const CAP: usize> {
    buf: [MaybeUninit<T>; CAP],
    rp: usize,
    wp: usize,
    full: bool,
}

impl<T, const CAP: usize> RingBuffer<T, CAP> {
    pub const fn new() -> RingBuffer<T, CAP> {
        RingBuffer {
            buf: unsafe { MaybeUninit::uninit().assume_init() },
            rp: 0,
            wp: 0,
            full: false,
        }
    }

    pub fn is_writable(&self) -> bool {
        !self.full
    }

    pub fn is_readable(&self) -> bool {
        self.full || self.rp != self.wp
    }

    pub fn push(&mut self, data: T) -> Result<(), T>
    where
        T: Copy,
    {
        if self.push_slice(&[data]) == 0 {
            Err(data)
        } else {
            Ok(())
        }
    }

    pub fn pop(&mut self) -> Option<T>
    where
        T: Copy,
    {
        self.pop_slice(1).map(|slice| slice[0])
    }

    pub fn push_slice(&mut self, data: &[T]) -> usize
    where
        T: Copy,
    {
        if !self.is_writable() || data.is_empty() {
            return 0;
        }

        let written_len = if self.wp >= self.rp {
            let free1 = self.wp..CAP;
            let free2 = 0..self.rp;
            let src1 = &data[..min(data.len(), free1.len())];
            let src2 = &data[src1.len()..min(data.len(), src1.len() + free2.len())];
            let dst1 = free1.start..(free1.start + src1.len());
            let dst2 = free2.start..(free2.start + src2.len());
            self.slice_mut(dst1).copy_from_slice(src1);
            self.slice_mut(dst2).copy_from_slice(src2);
            src1.len() + src2.len()
        } else {
            let free = self.wp..self.rp;
            let src = &data[..min(data.len(), free.len())];
            let dst = free.start..(free.start + src.len());
            self.slice_mut(dst).copy_from_slice(src);
            src.len()
        };

        self.wp = (self.wp + written_len) % CAP;
        self.full = self.wp == self.rp;
        written_len
    }

    pub fn pop_slice(&mut self, len: usize) -> Option<&[T]> {
        if !self.is_readable() {
            return None;
        }

        let range = if self.rp < self.wp {
            self.rp..min(self.rp + len, self.wp)
        } else {
            self.wp..min(self.wp + len, CAP)
        };

        self.rp = (self.rp + range.len()) % CAP;
        self.full = false;
        Some(self.slice(range))
    }

    fn slice(&self, range: Range<usize>) -> &[T] {
        debug_assert!(range.end <= CAP);
        unsafe {
            let ptr = self.buf.as_ptr() as *const T;
            slice::from_raw_parts(ptr.add(range.start), range.end - range.start)
        }
    }

    fn slice_mut(&mut self, range: Range<usize>) -> &mut [T] {
        debug_assert!(range.end <= CAP);
        unsafe {
            let ptr = self.buf.as_mut_ptr() as *mut T;
            slice::from_raw_parts_mut(ptr.add(range.start), range.end - range.start)
        }
    }
}

#[cfg(all(test, not(feature = "no_std")))]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut rb = RingBuffer::<u8, 8>::new();

        assert_eq!(rb.push_slice(b"abcd"), 4);
        assert_eq!(rb.slice(0..4), b"abcd");
        assert_eq!(rb.wp, 4);
        assert_eq!(rb.rp, 0);

        assert_eq!(rb.push_slice(b"1234"), 4);
        assert_eq!(rb.slice(0..8), b"abcd1234");

        // The buffer is full.
        assert_eq!(rb.push_slice(b"x"), 0);
        assert_eq!(rb.slice(0..8), b"abcd1234");
        assert_eq!(rb.wp, 0);
        assert_eq!(rb.rp, 0);

        assert_eq!(rb.pop_slice(3), Some("abc".as_bytes()));
        assert_eq!(rb.wp, 0);
        assert_eq!(rb.rp, 3);

        assert_eq!(rb.push_slice(b"xyz"), 3);
        assert_eq!(rb.slice(0..8), b"xyzd1234");
        assert_eq!(rb.wp, 3);
        assert_eq!(rb.rp, 3);
    }

    #[test]
    fn rp_wrapping() {
        let mut rb = RingBuffer::<u8, 4>::new();

        assert_eq!(rb.push_slice(b"abc"), 3);
        assert_eq!(rb.slice(0..3), b"abc");
        assert_eq!(rb.wp, 3);
        assert_eq!(rb.rp, 0);

        assert_eq!(rb.pop_slice(2), Some("ab".as_bytes()));
        assert_eq!(rb.wp, 3);
        assert_eq!(rb.rp, 2);

        assert_eq!(rb.push_slice(b"123"), 3);
        assert_eq!(rb.slice(0..4), b"23c1");
        assert_eq!(rb.wp, 2);
        assert_eq!(rb.rp, 2);
    }
}

use crate::result::{Errno, Error, Result};
use core::fmt;
use penguin_utils::alignment::align_down;

/// The base virtual address of straight mapping.
pub const KERNEL_BASE_ADDR: u64 = 0xffff_8000_0000_0000;

/// The end of straight mapping. Any physical address `P` is mapped into the
/// kernel's virtual memory address `KERNEL_BASE_ADDR + P`.
const KERNEL_STRAIGHT_MAP_PADDR_END: u64 = 0x1_0000_0000;

/// Represents a physical memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct PAddr(u64);

impl PAddr {
    pub const fn new(addr: usize) -> PAddr {
        PAddr(addr as u64)
    }

    pub const fn as_vaddr(self) -> VAddr {
        debug_assert!(self.0 < KERNEL_STRAIGHT_MAP_PADDR_END);
        VAddr::new((self.0 + KERNEL_BASE_ADDR) as usize)
    }

    pub const unsafe fn as_ptr<T>(self) -> *const T {
        debug_assert!(self.0 < KERNEL_STRAIGHT_MAP_PADDR_END);
        (self.0 + KERNEL_BASE_ADDR) as *const _
    }

    pub const unsafe fn as_mut_ptr<T>(self) -> *mut T {
        debug_assert!(self.0 < KERNEL_STRAIGHT_MAP_PADDR_END);
        (self.0 + KERNEL_BASE_ADDR) as *mut _
    }

    #[inline(always)]
    #[must_use]
    pub const fn add(self, offset: usize) -> PAddr {
        PAddr(self.0 + offset as u64)
    }

    #[inline(always)]
    pub const fn value(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for PAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.value())
    }
}

/// Represents a *kernel* virtual memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct VAddr(u64);

impl VAddr {
    pub const fn new(addr: usize) -> VAddr {
        debug_assert!(addr as u64 >= KERNEL_BASE_ADDR);
        VAddr(addr as u64)
    }

    pub const unsafe fn as_ptr<T>(self) -> *const T {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        self.0 as *const _
    }

    pub const unsafe fn as_mut_ptr<T>(self) -> *mut T {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        self.0 as *mut _
    }

    pub fn write_bytes(self, buf: &[u8]) {
        debug_assert!(self.value().checked_add(buf.len()).is_some());
        unsafe {
            self.as_mut_ptr::<u8>().copy_from(buf.as_ptr(), buf.len());
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn add(self, offset: usize) -> VAddr {
        VAddr::new(self.0 as usize + offset)
    }

    #[inline(always)]
    #[must_use]
    pub const fn sub(self, offset: usize) -> VAddr {
        VAddr::new(self.0 as usize - offset)
    }

    #[inline(always)]
    #[must_use]
    pub const fn align_down(self, alignment: usize) -> VAddr {
        VAddr::new(align_down(self.0 as usize, alignment))
    }

    #[inline(always)]
    pub const fn value(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for VAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.value())
    }
}

/// Represents a user virtual memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct UserVAddr(u64);

impl UserVAddr {
    pub const fn new(addr: usize) -> Result<UserVAddr> {
        if (addr as u64) >= KERNEL_BASE_ADDR {
            return Err(Error::with_message(Errno::EFAULT, "invalid user pointer"));
        }

        Ok(UserVAddr(addr as u64))
    }

    pub const unsafe fn new_unchecked(addr: usize) -> UserVAddr {
        UserVAddr(addr as u64)
    }

    #[inline(always)]
    #[must_use]
    pub const fn add(self, offset: usize) -> Result<UserVAddr> {
        UserVAddr::new(self.0 as usize + offset)
    }

    #[inline(always)]
    #[must_use]
    pub const fn sub(self, offset: usize) -> Result<UserVAddr> {
        UserVAddr::new(self.0 as usize - offset)
    }

    #[inline(always)]
    pub const fn value(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for UserVAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.value())
    }
}

use crate::result::{Errno, Error, Result};
use core::{
    fmt,
    mem::{size_of, MaybeUninit},
    ptr, slice,
};
use kerla_utils::alignment::align_down;

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

    #[inline(always)]
    pub const fn is_null(self) -> bool {
        self.0 == 0
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

    pub const fn as_paddr(self) -> PAddr {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        PAddr::new((self.0 - KERNEL_BASE_ADDR) as usize)
    }

    pub const unsafe fn as_ptr<T>(self) -> *const T {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        self.0 as *const _
    }

    pub const unsafe fn as_mut_ptr<T>(self) -> *mut T {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        self.0 as *mut _
    }

    pub unsafe fn read_volatile<T: Copy>(self) -> T {
        ptr::read_volatile(self.as_ptr::<T>())
    }

    pub unsafe fn write_volatile<T: Copy>(self, value: T) {
        ptr::write_volatile(self.as_mut_ptr(), value);
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

extern "C" {
    fn copy_from_user(dst: *mut u8, src: *const u8, len: usize);
    fn strncpy_from_user(dst: *mut u8, src: *const u8, max_len: usize) -> usize;
    fn copy_to_user(dst: *mut u8, src: *const u8, len: usize);
    fn memset_user(dst: *mut u8, value: u8, len: usize);
}

/// Represents a user virtual memory address.
///
/// It is guaranteed that `UserVaddr` contains a valid address, in other words,
/// it does not point to a kernel address.
///
/// Futhermore, like `NonNull<T>`, it is always non-null. Use `Option<UserVaddr>`
/// represent a nullable user pointer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct UserVAddr(u64);

impl UserVAddr {
    pub const fn new(addr: usize) -> Result<Option<UserVAddr>> {
        if (addr as u64) >= KERNEL_BASE_ADDR {
            return Err(Error::with_message(Errno::EFAULT, "invalid user pointer"));
        }

        if addr == 0 {
            Ok(None)
        } else {
            Ok(Some(UserVAddr(addr as u64)))
        }
    }

    pub const fn new_nonnull(addr: usize) -> Result<UserVAddr> {
        if (addr as u64) >= KERNEL_BASE_ADDR {
            return Err(Error::with_message(Errno::EFAULT, "invalid user pointer"));
        }

        if addr == 0 {
            return Err(Error::with_message(Errno::EFAULT, "null user pointer"));
        }

        Ok(UserVAddr(addr as u64))
    }

    pub const unsafe fn new_unchecked(addr: usize) -> UserVAddr {
        UserVAddr(addr as u64)
    }

    #[inline(always)]
    pub const fn as_isize(self) -> isize {
        // This cast is always safe thanks to the KERNEL_BASE_ADDR check in
        // `UserVAddr::new`.
        self.0 as isize
    }

    #[inline(always)]
    pub const fn add(self, offset: usize) -> UserVAddr {
        unsafe { UserVAddr::new_unchecked(self.0 as usize + offset) }
    }

    #[inline(always)]
    pub const fn sub(self, offset: usize) -> UserVAddr {
        unsafe { UserVAddr::new_unchecked(self.0 as usize - offset) }
    }

    #[inline(always)]
    pub const fn value(self) -> usize {
        self.0 as usize
    }

    pub fn access_ok(self, len: usize) -> Result<()> {
        match self.value().checked_add(len) {
            Some(end) if end <= KERNEL_BASE_ADDR as usize => Ok(()),
            Some(_end) => Err(Error::with_message(Errno::EFAULT, "invalid user pointer")),
            None => Err(Error::with_message(Errno::EFAULT, "overflow in access_ok")),
        }
    }

    pub fn read<T>(self) -> Result<T> {
        let mut buf: MaybeUninit<T> = MaybeUninit::uninit();
        self.read_bytes(unsafe {
            slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, size_of::<T>())
        })?;
        Ok(unsafe { buf.assume_init() })
    }

    pub fn read_bytes(self, buf: &mut [u8]) -> Result<()> {
        self.access_ok(buf.len())?;
        unsafe {
            copy_from_user(buf.as_mut_ptr(), self.value() as *const u8, buf.len());
        }
        Ok(())
    }

    /// Reads a string from the userspace and returns number of copied characters
    /// excluding the NUL character.
    ///
    /// Unlike strcnpy, **`dst` is NOT terminated by NULL**.
    pub fn read_cstr(self, buf: &mut [u8]) -> Result<usize> {
        self.access_ok(buf.len())?;
        let read_len =
            unsafe { strncpy_from_user(buf.as_mut_ptr(), self.value() as *const u8, buf.len()) };
        Ok(read_len)
    }

    pub fn write<T>(self, buf: &T) -> Result<usize> {
        let len = size_of::<T>();
        self.write_bytes(unsafe { slice::from_raw_parts(buf as *const T as *const u8, len) })?;
        Ok(len)
    }

    pub fn write_bytes(self, buf: &[u8]) -> Result<usize> {
        self.access_ok(buf.len())?;
        unsafe {
            copy_to_user(self.value() as *mut u8, buf.as_ptr(), buf.len());
        }
        Ok(buf.len())
    }

    pub fn fill(self, value: u8, len: usize) -> Result<usize> {
        self.access_ok(len)?;
        unsafe {
            memset_user(self.value() as *mut u8, value, len);
        }
        Ok(len)
    }
}

impl fmt::Display for UserVAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.value())
    }
}

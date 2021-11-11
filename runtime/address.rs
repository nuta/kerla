use crate::arch::{KERNEL_BASE_ADDR, KERNEL_STRAIGHT_MAP_PADDR_END};

#[cfg(debug_assertions)]
use crate::handler;

use core::{
    fmt,
    mem::{size_of, MaybeUninit},
    ptr, slice,
};
use kerla_utils::alignment::align_down;

/// Represents a physical memory address.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct PAddr(usize);

impl PAddr {
    pub const fn new(addr: usize) -> PAddr {
        PAddr(addr)
    }

    #[inline(always)]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    pub const fn as_vaddr(self) -> VAddr {
        debug_assert!(self.0 < KERNEL_STRAIGHT_MAP_PADDR_END);
        VAddr::new(self.0 + KERNEL_BASE_ADDR)
    }

    pub const fn as_ptr<T>(self) -> *const T {
        debug_assert!(self.0 < KERNEL_STRAIGHT_MAP_PADDR_END);
        (self.0 + KERNEL_BASE_ADDR) as *const _
    }

    pub const fn as_mut_ptr<T>(self) -> *mut T {
        debug_assert!(self.0 < KERNEL_STRAIGHT_MAP_PADDR_END);
        (self.0 + KERNEL_BASE_ADDR) as *mut _
    }

    #[inline(always)]
    #[must_use]
    pub const fn add(self, offset: usize) -> PAddr {
        PAddr(self.0 + offset)
    }

    #[inline(always)]
    pub const fn value(self) -> usize {
        self.0
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
pub struct VAddr(usize);

impl VAddr {
    pub const fn new(addr: usize) -> VAddr {
        debug_assert!(addr >= KERNEL_BASE_ADDR);
        VAddr(addr)
    }

    pub const fn as_paddr(self) -> PAddr {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        PAddr::new(self.0 - KERNEL_BASE_ADDR)
    }

    pub const fn is_accessible_from_kernel(addr: usize) -> bool {
        (addr) >= KERNEL_BASE_ADDR && (addr) < KERNEL_BASE_ADDR + KERNEL_STRAIGHT_MAP_PADDR_END
    }

    pub const fn as_ptr<T>(self) -> *const T {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        self.0 as *const _
    }

    pub const fn as_mut_ptr<T>(self) -> *mut T {
        debug_assert!(self.0 >= KERNEL_BASE_ADDR);
        self.0 as *mut _
    }

    /// # Safety
    /// See <https://doc.rust-lang.org/std/ptr/fn.read_volatile.html>.
    pub unsafe fn read_volatile<T: Copy>(self) -> T {
        ptr::read_volatile(self.as_ptr::<T>())
    }

    /// # Safety
    /// See <https://doc.rust-lang.org/std/ptr/fn.write_volatile.html>.
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
        VAddr::new(self.0 + offset)
    }

    #[inline(always)]
    #[must_use]
    pub const fn sub(self, offset: usize) -> VAddr {
        VAddr::new(self.0 - offset)
    }

    #[inline(always)]
    #[must_use]
    pub const fn align_down(self, alignment: usize) -> VAddr {
        VAddr::new(align_down(self.0, alignment))
    }

    #[inline(always)]
    pub const fn value(self) -> usize {
        self.0
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

fn call_usercopy_hook() {
    #[cfg(debug_assertions)]
    handler().usercopy_hook();
}

#[derive(Debug)]
pub struct AccessError;

#[derive(Debug)]
pub struct NullUserPointerError;

/// Represents a user virtual memory address.
///
/// It is guaranteed that `UserVaddr` contains a valid address, in other words,
/// it does not point to a kernel address.
///
/// Futhermore, like `NonNull<T>`, it is always non-null. Use `Option<UserVaddr>`
/// represent a nullable user pointer.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct UserVAddr(usize);

impl UserVAddr {
    pub const fn new(addr: usize) -> Option<UserVAddr> {
        if addr == 0 {
            None
        } else {
            Some(UserVAddr(addr))
        }
    }

    pub const fn new_nonnull(addr: usize) -> Result<UserVAddr, NullUserPointerError> {
        match UserVAddr::new(addr) {
            Some(uaddr) => Ok(uaddr),
            None => Err(NullUserPointerError),
        }
    }

    /// # Safety
    /// Make sure `addr` doesn't point to the kernel memory address or it can
    /// lead to a serious vulnerability!
    pub const unsafe fn new_unchecked(addr: usize) -> UserVAddr {
        UserVAddr(addr)
    }

    #[inline(always)]
    pub const fn as_isize(self) -> isize {
        // This cast is always safe thanks to the KERNEL_BASE_ADDR check in
        // `UserVAddr::new`.
        self.0 as isize
    }

    #[inline(always)]
    pub const fn add(self, offset: usize) -> UserVAddr {
        unsafe { UserVAddr::new_unchecked(self.0 + offset) }
    }

    #[inline(always)]
    pub const fn sub(self, offset: usize) -> UserVAddr {
        unsafe { UserVAddr::new_unchecked(self.0 - offset) }
    }

    #[inline(always)]
    pub const fn value(self) -> usize {
        self.0
    }

    pub fn access_ok(self, len: usize) -> Result<(), AccessError> {
        match self.value().checked_add(len) {
            Some(end) if end <= KERNEL_BASE_ADDR => Ok(()),
            Some(_end) => Err(AccessError),
            // Overflow.
            None => Err(AccessError),
        }
    }

    pub fn read<T>(self) -> Result<T, AccessError> {
        let mut buf: MaybeUninit<T> = MaybeUninit::uninit();
        self.read_bytes(unsafe {
            slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, size_of::<T>())
        })?;
        Ok(unsafe { buf.assume_init() })
    }

    pub fn read_bytes(self, buf: &mut [u8]) -> Result<(), AccessError> {
        call_usercopy_hook();
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
    pub fn read_cstr(self, buf: &mut [u8]) -> Result<usize, AccessError> {
        call_usercopy_hook();
        self.access_ok(buf.len())?;
        let read_len =
            unsafe { strncpy_from_user(buf.as_mut_ptr(), self.value() as *const u8, buf.len()) };
        Ok(read_len)
    }

    pub fn write<T>(self, buf: &T) -> Result<usize, AccessError> {
        let len = size_of::<T>();
        self.write_bytes(unsafe { slice::from_raw_parts(buf as *const T as *const u8, len) })?;
        Ok(len)
    }

    pub fn write_bytes(self, buf: &[u8]) -> Result<usize, AccessError> {
        call_usercopy_hook();
        self.access_ok(buf.len())?;
        unsafe {
            copy_to_user(self.value() as *mut u8, buf.as_ptr(), buf.len());
        }
        Ok(buf.len())
    }

    pub fn fill(self, value: u8, len: usize) -> Result<usize, AccessError> {
        call_usercopy_hook();
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

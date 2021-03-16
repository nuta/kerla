use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use x86::current::rflags::{self, RFlags};

pub struct SpinLock<T: ?Sized> {
    inner: spin::Mutex<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> SpinLock<T> {
        SpinLock {
            inner: spin::Mutex::new(value),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        SpinLockGuard {
            inner: ManuallyDrop::new(self.inner.lock()),
            rflags: unsafe { rflags::read() },
        }
    }
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

pub struct SpinLockGuard<'a, T: ?Sized> {
    inner: ManuallyDrop<spin::MutexGuard<'a, T>>,
    rflags: RFlags,
}

impl<'a, T: ?Sized> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.inner);
            rflags::set(rflags::read() | (self.rflags & rflags::RFlags::FLAGS_IF));
        }
    }
}

impl<'a, T: ?Sized> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.inner
    }
}

impl<'a, T: ?Sized> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.inner
    }
}

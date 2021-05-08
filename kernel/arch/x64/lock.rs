use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use x86::current::rflags::{self, RFlags};

use crate::printk::backtrace;

pub struct SpinLock<T: ?Sized> {
    pub inner: spin::mutex::SpinMutex<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> SpinLock<T> {
        SpinLock {
            inner: spin::mutex::SpinMutex::new(value),
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        if self.inner.is_locked() {
            // Since we don't yet support multiprocessors and interrupts are
            // disabled until all locks are released, `lock()` will never fail
            // unless a dead lock has occurred.
            //
            // TODO: Remove when we got SMP support.
            debug_warn!("already locked");
            backtrace();
        }

        let rflags = unsafe { rflags::read() };
        unsafe {
            asm!("cli");
        }

        SpinLockGuard {
            inner: ManuallyDrop::new(self.inner.lock()),
            rflags,
        }
    }

    pub unsafe fn force_unlock(&self) {
        self.inner.force_unlock()
    }
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

pub struct SpinLockGuard<'a, T: ?Sized> {
    inner: ManuallyDrop<spin::mutex::SpinMutexGuard<'a, T>>,
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

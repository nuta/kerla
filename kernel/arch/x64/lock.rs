use core::ops::{Deref, DerefMut};

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
            inner: self.inner.lock(),
        }
    }
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

pub struct SpinLockGuard<'a, T: ?Sized> {
    inner: spin::MutexGuard<'a, T>,
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

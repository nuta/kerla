use cfg_if::cfg_if;
use core::arch::asm;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

use crate::arch::SavedInterruptStatus;
use crate::backtrace::backtrace;

#[cfg(debug_assertions)]
use crate::backtrace::CapturedBacktrace;
#[cfg(debug_assertions)]
use crate::global_allocator::is_kernel_heap_enabled;
#[cfg(debug_assertions)]
use atomic_refcell::AtomicRefCell;

pub struct SpinLock<T: ?Sized> {
    #[cfg(debug_assertions)]
    locked_by: AtomicRefCell<Option<CapturedBacktrace>>,
    inner: spin::mutex::SpinMutex<T>,
}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> SpinLock<T> {
        SpinLock {
            inner: spin::mutex::SpinMutex::new(value),
            #[cfg(debug_assertions)]
            locked_by: AtomicRefCell::new(None),
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
            cfg_if! {
                if #[cfg(debug_assertions)] {
                    let trace = self.locked_by.borrow();
                    if let Some(trace) = trace.as_ref() {
                        debug_warn!(
                            "DEAD LOCK: already locked from the following context\n{:?}",
                            trace
                        );
                    } else {
                        debug_warn!("DEAD LOCK: already locked");
                    }
                } else {
                    debug_warn!("DEAD LOCK: already locked");
                }
            }

            debug_warn!("Tried to lock from:");
            backtrace();
        }

        let saved_intr_status = SavedInterruptStatus::save();
        unsafe {
            asm!("cli");
        }

        let guard = self.inner.lock();

        #[cfg(debug_assertions)]
        if is_kernel_heap_enabled() {
            *self.locked_by.borrow_mut() = Some(CapturedBacktrace::capture());
        }

        SpinLockGuard {
            inner: ManuallyDrop::new(guard),
            saved_intr_status: ManuallyDrop::new(saved_intr_status),
            #[cfg(debug_assertions)]
            locked_by: &self.locked_by,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

pub struct SpinLockGuard<'a, T: ?Sized> {
    inner: ManuallyDrop<spin::mutex::SpinMutexGuard<'a, T>>,
    #[cfg(debug_assertions)]
    locked_by: &'a AtomicRefCell<Option<CapturedBacktrace>>,
    saved_intr_status: ManuallyDrop<SavedInterruptStatus>,
}

impl<'a, T: ?Sized> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.inner);
        }

        cfg_if! {
            if #[cfg(debug_assertions)] {
                *self.locked_by.borrow_mut() = None;
            }
        }

        unsafe {
            ManuallyDrop::drop(&mut self.saved_intr_status);
        }
    }
}

impl<'a, T: ?Sized> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<'a, T: ?Sized> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

use super::*;
use crate::arch::{self, enable_interrupt, is_interrupt_enabled};
use crate::process::PId;

use alloc::sync::Arc;

use arrayvec::ArrayVec;

use core::mem::{self};

cpu_local! {
    static ref HELD_LOCKS: ArrayVec<[Arc<Process>; 2]> = ArrayVec::new();
}

/// Yields execution to another thread. When the currently running thread is resumed
// in future, it will be
pub fn switch() {
    // Save the current interrupt enable flag to restore it in the next execution
    // of the currently running thread.
    let interrupt_enabled = is_interrupt_enabled();

    let prev_thread = CURRENT.get();
    let next_thread = {
        let scheduler = SCHEDULER.lock();

        // Push back the currently running thread to the runqueue if it's still
        // ready for running, in other words, it's not blocked.
        if prev_thread.pid != PId::new(0) && prev_thread.state() == ProcessState::Runnable {
            scheduler.enqueue((*prev_thread).clone());
        }

        // Pick a thread to run next.
        match scheduler.pick_next() {
            Some(next) => next,
            None => IDLE_THREAD.get().get().clone(),
        }
    };

    debug_assert!(next_thread.state() == ProcessState::Runnable);

    if Arc::ptr_eq(prev_thread, &next_thread) {
        // Continue executing the current process.
        return;
    }

    // Save locks that will be released later.
    debug_assert!(HELD_LOCKS.get().is_empty());
    HELD_LOCKS.as_mut().push((*prev_thread).clone());
    HELD_LOCKS.as_mut().push(next_thread.clone());

    // Since these locks won't be dropped until the current (prev) thread is
    // resumed next time, we'll unlock these locks in `after_switch` in the next
    // thread's context.
    let mut prev_arch = prev_thread.arch.lock();
    let mut next_arch = next_thread.arch.lock();

    if let Some(vm) = next_thread.vm.as_ref() {
        let lock = vm.lock();
        lock.page_table().switch();
    }

    // Switch into the next thread.
    CURRENT.as_mut().set(next_thread.clone());
    arch::switch_thread(&mut *prev_arch, &mut *next_arch);

    // Don't call destructors as they're unlocked in `after_switch`.
    mem::forget(prev_arch);
    mem::forget(next_arch);

    // Now we're in the next thread. Release held locks and continue executing.
    after_switch();

    // Retstore the interrupt enable flag manually because lock guards
    // (`prev` and `next`) that holds the flag state are `mem::forget`-ed.
    if interrupt_enabled {
        unsafe {
            enable_interrupt();
        }
    }
}

#[no_mangle]
pub extern "C" fn after_switch() {
    for thread in HELD_LOCKS.as_mut().drain(..) {
        unsafe {
            thread.arch.force_unlock();
        }
    }
}

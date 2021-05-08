use super::*;
use crate::process::PId;
use crate::{
    arch::{self, enable_interrupt, is_interrupt_enabled},
    process::process::PROCESSES,
};

use alloc::sync::Arc;

use arrayvec::ArrayVec;

use core::mem::{self};

cpu_local! {
    static ref HELD_LOCKS: ArrayVec<Arc<SpinLock<Process>>, 2> = ArrayVec::new_const();
}

/// Yields execution to another thread.
pub fn switch() {
    // Save the current interrupt enable flag to restore it in the next execution
    // of the currently running thread.
    let interrupt_enabled = is_interrupt_enabled();

    let prev_proc = current_process_arc().clone();
    let (prev_pid, prev_state) = {
        let prev = prev_proc.lock();
        (prev.pid(), prev.state)
    };
    let next_proc = {
        let scheduler = SCHEDULER.lock();

        // Push back the currently running thread to the runqueue if it's still
        // ready for running, in other words, it's not blocked.
        if prev_pid != PId::new(0) && prev_state == ProcessState::Runnable {
            scheduler.enqueue(prev_pid);
        }

        // Pick a thread to run next.
        match scheduler.pick_next() {
            Some(next_pid) => PROCESSES.lock().get(&next_pid).unwrap().clone(),
            None => IDLE_THREAD.get().get().clone(),
        }
    };

    if Arc::ptr_eq(&prev_proc, &next_proc) {
        // Continue executing the current process.
        return;
    }

    let mut prev = prev_proc.lock();
    let mut next = next_proc.lock();
    debug_assert!(next.state() == ProcessState::Runnable);

    // Save locks that will be released later.
    debug_assert!(HELD_LOCKS.get().is_empty());
    HELD_LOCKS.as_mut().push(prev_proc.clone());
    HELD_LOCKS.as_mut().push(next_proc.clone());

    if let Some(vm) = next.vm.as_ref() {
        let lock = vm.lock();
        lock.page_table().switch();
    }

    // Drop `next_thread` here because `switch_thread` won't return when the current
    // process is being destroyed (e.g. by exit(2)) and it leads to a memory leak.
    //
    // To cheat the borrow checker we do so by `Arc::decrement_strong_count`.
    debug_assert!(Arc::strong_count(&next_proc) > 1);
    unsafe {
        Arc::decrement_strong_count(Arc::as_ptr(&prev_proc));
        Arc::decrement_strong_count(Arc::as_ptr(&next_proc));
    }

    // Switch into the next thread.
    CURRENT.as_mut().set(next_proc.clone());
    arch::switch_thread(&mut prev.arch, &mut next.arch);

    // Don't call destructors: we'll unlock these locks in `after_switch` in the
    // next thread's context because these lock guards won't be dropped until the
    // current (prev) thread is resumed next time.
    mem::forget(prev);
    mem::forget(next);

    // Don't call destructors as we've already decremented (dropped) the
    // reference count by `Arc::decrement_strong_count` above.
    mem::forget(prev_proc);
    mem::forget(next_proc);

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
    for proc in HELD_LOCKS.as_mut().drain(..) {
        unsafe {
            proc.force_unlock();
        }
    }
}

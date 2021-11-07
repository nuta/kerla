use super::*;
use crate::process::PId;
use crate::{
    arch::{self},
    process::process::PROCESSES,
};

use alloc::sync::Arc;

use core::mem::{self};

/// Yields execution to another thread.
pub fn switch() {
    let prev = current_process().clone();
    let prev_pid = prev.pid();
    let prev_state = prev.state();
    let next = {
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

    if Arc::ptr_eq(&prev, &next) {
        // Continue executing the current process.
        return;
    }

    debug_assert!(next.state() == ProcessState::Runnable);

    if let Some(vm) = next.vm().clone() {
        let lock = vm.lock();
        lock.page_table().switch();
    }

    // Drop `prev` and `next` here because `switch_thread` won't return when the
    // current process is being destroyed (e.g. by exit(2)).
    //
    // Since processes are referenced from at least the following two places,
    // we can safely decrement reference counts without immediately drop here:
    //
    // - prev or next: they holds Arc<Process> (not &Arc<Process>).
    // - Their parent processes's list of children.
    //
    // To cheat the borrow checker we do so by `Arc::decrement_strong_count`.
    debug_assert!(Arc::strong_count(&prev) > 1);
    debug_assert!(Arc::strong_count(&next) > 1);
    unsafe {
        Arc::decrement_strong_count(Arc::as_ptr(&prev));
        Arc::decrement_strong_count(Arc::as_ptr(&next));
    }

    // Switch into the next thread.
    CURRENT.as_mut().set(next.clone());
    arch::switch_thread(prev.arch(), next.arch());

    // Don't call destructors as we've already decremented (dropped) the
    // reference count by `Arc::decrement_strong_count` above.
    mem::forget(prev);
    mem::forget(next);
}

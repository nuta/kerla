use super::{current_process, current_process_arc, switch, Process, ProcessState};
use crate::result::Result;
use crate::{arch::SpinLock, result::Errno};

use alloc::{collections::VecDeque, sync::Arc};

pub struct WaitQueue {
    queue: SpinLock<VecDeque<Arc<SpinLock<Process>>>>,
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue {
            queue: SpinLock::new(VecDeque::new()),
        }
    }

    /// Sleeps on the wait queue until `sleep_if_none` returns `Some`.
    ///
    /// If a signal is arrived, this method returns `Err(Errno::EINTR)`.
    pub fn sleep_signalable_until<F, R>(&self, mut sleep_if_none: F) -> Result<R>
    where
        F: FnMut() -> Result<Option<R>>,
    {
        loop {
            // Enqueue the current process into the wait queue before checking
            // if we need to sleep on it.
            //
            // You might wonder why we don't `sleep_if_none` first. Consider
            // the following situation:
            //
            //  1. Check the RX packets queue and it's now empty, the current
            //     thread needs to sleep until we receive a new packet:
            //     `sleep_if_none` returns None.
            //
            //  [an interrupt arrives here]: receive a RX packet from the device.
            //
            //  3. Enqueue the current thread into the wait queue.
            //  4. Enter the sleep state despite a RX packet exists on the queue!
            current_process().set_state(ProcessState::BlockedSignalable);
            self.queue.lock().push_back(current_process_arc().clone());

            if current_process().has_pending_signals() {
                current_process().resume();
                self.queue
                    .lock()
                    .retain(|proc| !Arc::ptr_eq(&proc, current_process_arc()));
                return Err(Errno::EINTR.into());
            }

            let ret_value = match sleep_if_none() {
                Ok(Some(ret_value)) => Some(Ok(ret_value)),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            };

            if let Some(ret_value) = ret_value {
                // The condition is met. The current thread doesn't have to sleep.
                current_process().resume();
                self.queue
                    .lock()
                    .retain(|proc| !Arc::ptr_eq(&proc, current_process_arc()));
                return ret_value;
            }

            // Run other threads until someone wake us up...
            switch();
        }
    }

    pub fn _wake_one(&self) {
        let mut queue = self.queue.lock();
        if let Some(process) = queue.pop_front() {
            process.lock().resume();
        }
    }

    pub fn wake_all(&self) {
        let mut queue = self.queue.lock();
        while let Some(process) = queue.pop_front() {
            process.lock().resume();
        }
    }
}

use crate::process::PId;
use alloc::collections::VecDeque;
use kerla_runtime::spinlock::SpinLock;

/// The process scheduler.
///
/// Currently, it implements a round-robin algorithm.
pub struct Scheduler {
    run_queue: SpinLock<VecDeque<PId>>,
}

impl Scheduler {
    /// Creates a scheduler.
    pub fn new() -> Scheduler {
        Scheduler {
            run_queue: SpinLock::new(VecDeque::new()),
        }
    }

    /// Enqueues a process into the runqueue.
    pub fn enqueue(&self, pid: PId) {
        self.run_queue.lock().push_back(pid);
    }

    /// Returns the next process to run.
    ///
    /// The process is removed from the runqueue so you need to enqueue it by
    /// [`Scheduler::enqueue`] again.
    pub fn pick_next(&self) -> Option<PId> {
        self.run_queue.lock().pop_front()
    }

    /// Removes the process from the runqueue.
    pub fn remove(&self, pid: PId) {
        self.run_queue.lock().retain(|p| *p != pid);
    }
}

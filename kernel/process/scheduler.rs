use crate::arch::SpinLock;
use crate::process::PId;
use alloc::collections::VecDeque;

pub struct Scheduler {
    run_queue: SpinLock<VecDeque<PId>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            run_queue: SpinLock::new(VecDeque::new()),
        }
    }

    pub fn enqueue(&self, pid: PId) {
        self.run_queue.lock().push_back(pid);
    }

    pub fn pick_next(&self) -> Option<PId> {
        self.run_queue.lock().pop_front()
    }

    pub fn remove(&self, pid: PId) {
        self.run_queue.lock().retain(|p| *p != pid);
    }
}

use super::*;
use crate::arch::SpinLock;
use alloc::collections::VecDeque;

pub struct Scheduler {
    run_queue: SpinLock<VecDeque<Arc<Process>>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            run_queue: SpinLock::new(VecDeque::new()),
        }
    }

    pub fn enqueue(&self, thread: Arc<Process>) {
        self.run_queue.lock().push_back(thread);
    }

    pub fn pick_next(&self) -> Option<Arc<Process>> {
        self.run_queue.lock().pop_front()
    }

    pub fn remove(&self, thread: &Arc<Process>) {
        self.run_queue.lock().retain(|t| !Arc::ptr_eq(&t, &thread));
    }
}

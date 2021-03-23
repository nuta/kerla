use super::{current_process, switch, Process, ProcessState, SCHEDULER};
use crate::arch::SpinLock;
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub struct WaitQueue {
    queue: SpinLock<VecDeque<Arc<Process>>>,
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue {
            queue: SpinLock::new(VecDeque::new()),
        }
    }

    pub fn sleep(&self) {
        self.queue.lock().push_back(current_process().clone());
        switch(ProcessState::Sleeping);
    }

    pub fn wake_one(&self) {
        if let Some(process) = self.queue.lock().pop_front() {
            SCHEDULER.lock().enqueue(process);
        }
    }

    pub fn wake_all(&mut self) {
        for process in self.queue.lock().drain(..) {
            SCHEDULER.lock().enqueue(process);
        }
    }
}

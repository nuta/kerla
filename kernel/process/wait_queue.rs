use super::{current_process, switch, Process, ProcessState, SCHEDULER};
use crate::arch::SpinLock;
use alloc::sync::Arc;
use crossbeam::queue::SegQueue;

pub struct WaitQueue {
    queue: SegQueue<Arc<Process>>,
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue {
            queue: SegQueue::new(),
        }
    }

    pub fn sleep(&self) {
        self.queue.push(current_process().clone());
        switch(ProcessState::Sleeping);
    }

    pub fn wake_one(&self) {
        if let Some(process) = self.queue.pop() {
            SCHEDULER.lock().enqueue(process);
        }
    }

    pub fn wake_all(&self) {
        while let Some(process) = self.queue.pop() {
            SCHEDULER.lock().enqueue(process);
        }
    }
}

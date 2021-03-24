use super::*;
use crossbeam::queue::SegQueue;

pub struct Scheduler {
    run_queue: SegQueue<Arc<Process>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            run_queue: SegQueue::new(),
        }
    }

    pub fn enqueue(&self, thread: Arc<Process>) {
        self.run_queue.push(thread);
    }

    pub fn pick_next(&self) -> Option<Arc<Process>> {
        self.run_queue.pop()
    }
}

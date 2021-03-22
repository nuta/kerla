use super::*;

pub struct Scheduler {
    run_queue: VecDeque<Arc<Process>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            run_queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, thread: Arc<Process>) {
        self.run_queue.push_back(thread);
    }

    pub fn pick_next(&mut self) -> Option<Arc<Process>> {
        self.run_queue.pop_front()
    }
}

use crate::fs::inode::PollStatus;
use crate::prelude::*;
use crate::process::WaitQueue;

/// An epoll instance created by epoll_create(2). It's referred from the user
/// through a file descriptor.
pub struct EPoll {
    wq: WaitQueue,
}

#[derive(Clone)]
pub struct EPolledItem {
    epoll: Arc<EPoll>,
    events: PollStatus,
}

impl EPolledItem {
    pub fn wake_if_satisfied(&self, status: PollStatus) {
        // If any of the events in the `events` field is satisfied, wake up
        // waiting processes.
        if self.events.intersects(status) {
            self.epoll.wq.wake_all();
        }
    }
}

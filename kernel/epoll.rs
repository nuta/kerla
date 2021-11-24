use core::fmt;

use kerla_api::sync::SpinLock;

use crate::fs::inode::PollStatus;
use crate::fs::opened_file::{Fd, OpenedFile};
use crate::prelude::*;
use crate::process::WaitQueue;

/// The epoll instance referred from the user through a file descriptor.
pub struct EPoll {
    instance: Arc<EPollInstance>,
}

impl EPoll {
    pub fn add(&self, file: &Arc<OpenedFile>, fd: Fd, events: PollStatus) -> Result<()> {
        self.instance.add(file, fd, events)
    }

    pub fn del(&self, file: &Arc<OpenedFile>, fd: Fd) -> Result<()> {
        self.instance.del(file, fd)
    }
}

impl fmt::Debug for EPoll {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EPoll").finish()
    }
}

impl Drop for EPoll {
    fn drop(&mut self) {
        // Remove epoll items from FileLike implementations.
        self.instance.items.lock().clear();
    }
}

/// An epoll instance created by epoll_create(2).
pub struct EPollInstance {
    wq: WaitQueue,
    items: SpinLock<Vec<EPollItem>>,
}

impl EPollInstance {
    pub fn new() -> EPollInstance {
        EPollInstance {
            wq: WaitQueue::new(),
            items: SpinLock::new(Vec::new()),
        }
    }

    pub fn add(self: &Arc<Self>, file: &Arc<OpenedFile>, fd: Fd, events: PollStatus) -> Result<()> {
        let item = EPollItem::new(file, fd, self.clone(), events);
        self.items.lock().push(item);
        Ok(())
    }

    pub fn del(self: &Arc<Self>, file: &Arc<OpenedFile>, fd: Fd) -> Result<()> {
        let key = EPollItemKey {
            file: Arc::downgrade(file),
            fd,
        };
        self.items.lock().retain(|item| item.key != key);
        Ok(())
    }
}

/// A key used to determine an epoll item. This struct contains a pointer to a
/// file because a file descriptor can be reused for different files.
#[derive(Clone)]
pub struct EPollItemKey {
    fd: Fd,
    file: Weak<OpenedFile>,
}

impl PartialEq for EPollItemKey {
    fn eq(&self, other: &EPollItemKey) -> bool {
        self.fd == other.fd && self.file.ptr_eq(&other.file)
    }
}

/// Represents a file being watched from an epoll instance. Added and deleted by
/// epoll_ctl(2).
#[derive(Clone)]
pub struct EPollItem {
    key: EPollItemKey,
    epoll: Arc<EPollInstance>,
    events: PollStatus,
}

impl EPollItem {
    pub fn new(
        file: &Arc<OpenedFile>,
        fd: Fd,
        epoll: Arc<EPollInstance>,
        events: PollStatus,
    ) -> EPollItem {
        EPollItem {
            key: EPollItemKey {
                file: Arc::downgrade(file),
                fd,
            },
            epoll,
            events,
        }
    }

    pub fn notify_if_satisfied(&self, status: PollStatus) {
        // If any of the events in the `events` field is satisfied, wake up
        // waiting processes.
        if self.events.intersects(status) {
            self.epoll.wq.wake_all();
        }
    }
}

impl PartialEq for EPollItem {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Drop for EPollItem {
    fn drop(&mut self) {
        if let Some(opened_file) = self.key.file.upgrade() {
            warn_if_err!(opened_file.epoll_del(self));
        }
    }
}

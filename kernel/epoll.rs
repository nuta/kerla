use core::fmt;
use core::ops::BitAnd;

use kerla_api::sync::SpinLock;

use crate::ctypes::c_int;
use crate::fs::inode::{INode, PollStatus};
use crate::fs::opened_file::{Fd, OpenedFile};
use crate::prelude::*;
use crate::process::WaitQueue;

/// The epoll instance referred from the user through a file descriptor.
pub struct EPoll {
    instance: Arc<EPollInstance>,
}

impl EPoll {
    pub fn new() -> Arc<EPoll> {
        Arc::new(EPoll {
            instance: Arc::new(EPollInstance::new()),
        })
    }

    pub fn add(&self, file: &Arc<OpenedFile>, fd: Fd, events: PollStatus) -> Result<()> {
        self.instance.add(file, fd, events)
    }

    pub fn del(&self, file: &Arc<OpenedFile>, fd: Fd) -> Result<()> {
        self.instance.del(file, fd)
    }

    pub fn wait<F>(&self, _timeout: c_int, mut callback: F) -> Result<()>
    where
        F: FnMut(Fd, PollStatus) -> Result<()>,
    {
        // TODO: Support timeout
        self.instance.wq.sleep_signalable_until(|| {
            let mut events = self.instance.pending_events.lock();
            let mut new_events = Vec::new();
            let mut delivered_any = false;
            for e in events.drain(..) {
                let current_events = e.inode.as_file()?.poll()?;
                let actual_events = e.listened_events & current_events;
                if actual_events.is_empty() {
                    continue;
                }

                callback(e.fd, actual_events)?;
                delivered_any = true;

                if !e.listened_events.contains(PollStatus::EPOLLET) {
                    // If the emitted event is level-triggered, keep it in the
                    // pending list so that we can keep waking the process up
                    // until the event goes away.
                    new_events.push(e);
                }
            }

            *events = new_events;
            Ok(if delivered_any { Some(()) } else { None })
        })
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

struct EmittedEvent {
    fd: Fd,
    inode: INode,
    listened_events: PollStatus,
}

/// An epoll instance created by epoll_create(2).
pub struct EPollInstance {
    wq: WaitQueue,
    pending_events: SpinLock<Vec<EmittedEvent>>,
    items: SpinLock<Vec<EPollItem>>,
}

impl EPollInstance {
    pub fn new() -> EPollInstance {
        EPollInstance {
            wq: WaitQueue::new(),
            items: SpinLock::new(Vec::new()),
            pending_events: SpinLock::new(Vec::new()),
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

    fn notify(&self, e: EmittedEvent) {
        let mut pending_events = self.pending_events.lock();
        pending_events.push(e);
        self.wq.wake_all();
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
    instance: Arc<EPollInstance>,
    events: PollStatus,
    inode: INode,
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
            instance: epoll,
            events,
            inode: file.inode().clone(),
        }
    }

    pub fn fd(&self) -> Fd {
        self.key.fd
    }

    pub fn notify_if_satisfied(&self, status: PollStatus) {
        // If any of the events in the `events` field is satisfied, wake up
        // waiting processes.
        let events = self.events.bitand(status);
        if !events.is_empty() {
            self.instance.notify(EmittedEvent {
                fd: self.fd(),
                inode: self.inode.clone(),
                listened_events: self.events,
            });
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

/// `struct epoll_event`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EPollEvent {
    pub events: u32,
    pub data: EPollData,
}

/// `struct epoll_data`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union EPollData {
    pub ptr: usize,
    pub fd: c_int,
    pub u32: u32,
    pub u64: u64,
}

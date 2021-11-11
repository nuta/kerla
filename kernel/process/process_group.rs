use crate::prelude::*;
use alloc::{collections::BTreeMap, vec::Vec};
use kerla_runtime::spinlock::SpinLock;

use super::{signal::Signal, Process};

pub static PROCESS_GROUPS: SpinLock<BTreeMap<PgId, Arc<SpinLock<ProcessGroup>>>> =
    SpinLock::new(BTreeMap::new());

/// A process group ID.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PgId(i32);

impl PgId {
    pub const fn new(pgid: i32) -> PgId {
        PgId(pgid)
    }

    pub const fn as_i32(self) -> i32 {
        self.0
    }
}

/// A process group.
pub struct ProcessGroup {
    pgid: PgId,
    processes: Vec<Weak<Process>>,
}

impl ProcessGroup {
    /// Create a new process group.
    pub fn new(pgid: PgId) -> Arc<SpinLock<ProcessGroup>> {
        let pg = Arc::new(SpinLock::new(ProcessGroup {
            pgid,
            processes: Vec::new(),
        }));

        PROCESS_GROUPS.lock().insert(pgid, pg.clone());
        pg
    }

    /// Looks for the process group with the given process group ID. Returns
    /// `None` if it does not exist.
    pub fn find_by_pgid(pgid: PgId) -> Option<Arc<SpinLock<ProcessGroup>>> {
        PROCESS_GROUPS.lock().get(&pgid).cloned()
    }

    /// Looks for the process group with the given process group ID. If it does
    /// not exist, create a new process group.
    pub fn find_or_create_by_pgid(pgid: PgId) -> Arc<SpinLock<ProcessGroup>> {
        let pg = { PROCESS_GROUPS.lock().get(&pgid).cloned() };
        pg.unwrap_or_else(|| ProcessGroup::new(pgid))
    }

    /// The process group ID.
    pub fn pgid(&self) -> PgId {
        self.pgid
    }

    /// Adds a process into the group.
    pub fn add(&mut self, proc: Weak<Process>) {
        self.processes.push(proc);
    }

    /// Removes a process from the group.
    pub fn remove(&mut self, proc: &Weak<Process>) {
        self.processes.retain(|p| !Weak::ptr_eq(p, proc));
        if self.processes.is_empty() {
            PROCESS_GROUPS.lock().remove(&self.pgid);
        }
    }

    /// Removes processes that're dropped or being dropped.
    pub fn remove_dropped_processes(&mut self) {
        self.processes.retain(|proc| proc.upgrade().is_some());
        if self.processes.is_empty() {
            PROCESS_GROUPS.lock().remove(&self.pgid);
        }
    }

    /// Sends a signal to all processes in the proces group.
    pub fn signal(&mut self, signal: Signal) {
        for proc in &self.processes {
            proc.upgrade().unwrap().send_signal(signal);
        }
    }
}

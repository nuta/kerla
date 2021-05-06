use crate::arch::SpinLock;
use crate::prelude::*;
use alloc::{collections::BTreeMap, vec::Vec};

use super::{signal::Signal, Process};

pub static PROCESS_GROUPS: SpinLock<BTreeMap<PgId, Arc<SpinLock<ProcessGroup>>>> =
    SpinLock::new(BTreeMap::new());

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

pub struct ProcessGroup {
    pgid: PgId,
    processes: Vec<Weak<SpinLock<Process>>>,
}

impl ProcessGroup {
    pub fn new(pgid: PgId) -> Arc<SpinLock<ProcessGroup>> {
        let pg = Arc::new(SpinLock::new(ProcessGroup {
            pgid,
            processes: Vec::new(),
        }));

        PROCESS_GROUPS.lock().insert(pgid, pg.clone());
        pg
    }

    pub fn find_by_pgid(pgid: PgId) -> Option<Arc<SpinLock<ProcessGroup>>> {
        PROCESS_GROUPS.lock().get(&pgid).cloned()
    }

    pub fn find_or_create_by_pgid(pgid: PgId) -> Arc<SpinLock<ProcessGroup>> {
        let pg = { PROCESS_GROUPS.lock().get(&pgid).cloned() };
        pg.unwrap_or_else(|| ProcessGroup::new(pgid))
    }

    pub fn pgid(&self) -> PgId {
        self.pgid
    }

    pub fn add(&mut self, proc: Weak<SpinLock<Process>>) {
        self.processes.push(proc);
    }

    pub fn remove(&mut self, proc: &Weak<SpinLock<Process>>) {
        self.processes.retain(|p| !Weak::ptr_eq(p, proc));
        if self.processes.is_empty() {
            info!("REMOVE: {:?}", self.pgid);
            PROCESS_GROUPS.lock().remove(&self.pgid);
        }
    }

    pub fn remove_dropped_processes(&mut self) {
        self.processes.retain(|proc| proc.upgrade().is_some());
        if self.processes.is_empty() {
            PROCESS_GROUPS.lock().remove(&self.pgid);
        }
    }

    pub fn signal(&mut self, signal: Signal) {
        for proc in &self.processes {
            proc.upgrade().unwrap().lock().signal(signal);
        }
    }
}

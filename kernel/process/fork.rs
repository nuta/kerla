use crate::{arch::SpinLock, process::Process};
use crate::{arch::SyscallFrame, result::Result};
use alloc::sync::Arc;
use alloc::vec::Vec;
use atomic_refcell::AtomicRefCell;
use crossbeam::atomic::AtomicCell;

use super::{
    process::{alloc_pid, PROCESSES},
    signal::SignalDelivery,
    ProcessState, SCHEDULER,
};

/// Creates a new process. The calling process (`self`) will be the parent
/// process of the created process. Returns the created child process.
pub fn fork(parent: &Arc<Process>, parent_frame: &SyscallFrame) -> Result<Arc<Process>> {
    let parent_weak = Arc::downgrade(parent);
    let mut process_table = PROCESSES.lock();
    let pid = alloc_pid(&mut process_table)?;
    let arch = parent.arch.fork(parent_frame)?;
    let vm = parent.vm().as_ref().unwrap().lock().fork()?;
    let opened_files = parent.opened_files.lock().fork();
    let process_group = parent.process_group.clone();

    let child = Arc::new(Process {
        process_group: process_group.clone(),
        pid,
        state: AtomicCell::new(ProcessState::Runnable),
        parent: parent_weak,
        cmdline: parent.cmdline.clone(),
        children: SpinLock::new(Vec::new()),
        vm: AtomicRefCell::new(Some(Arc::new(SpinLock::new(vm)))),
        opened_files: Arc::new(SpinLock::new(opened_files)),
        root_fs: parent.root_fs.clone(),
        arch,
        signals: SpinLock::new(SignalDelivery::new()),
        signaled_frame: AtomicCell::new(None),
    });

    process_group
        .borrow()
        .upgrade()
        .unwrap()
        .lock()
        .add(Arc::downgrade(&child));
    parent.children.lock().push(child.clone());
    process_table.insert(pid, child.clone());
    SCHEDULER.lock().enqueue(pid);
    Ok(child)
}

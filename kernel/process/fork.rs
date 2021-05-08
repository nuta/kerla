use crate::{arch::SpinLock, process::Process};
use crate::{arch::SyscallFrame, result::Result};
use alloc::sync::Arc;
use alloc::vec::Vec;

use super::{
    process::{alloc_pid, PROCESSES},
    signal::SignalDelivery,
    ProcessState, SCHEDULER,
};

/// Creates a new process. The calling process (`self`) will be the parent
/// process of the created process. Returns the created child process.
pub fn fork(
    parent: &Arc<SpinLock<Process>>,
    parent_frame: &SyscallFrame,
) -> Result<Arc<SpinLock<Process>>> {
    let parent_weak = Arc::downgrade(parent);
    let mut parent = parent.lock();
    let mut process_table = PROCESSES.lock();
    let pid = alloc_pid(&mut process_table)?;
    let arch = parent.arch.fork(parent_frame)?;
    let vm = parent.vm.as_ref().unwrap().lock().fork()?;
    let opened_files = parent.opened_files.lock().fork();
    let process_group = parent.process_group.clone();

    let child = Arc::new(SpinLock::new(Process {
        process_group: process_group.clone(),
        pid,
        state: ProcessState::Runnable,
        parent: Some(parent_weak),
        children: Vec::new(),
        vm: Some(Arc::new(SpinLock::new(vm))),
        opened_files: Arc::new(SpinLock::new(opened_files)),
        root_fs: parent.root_fs.clone(),
        arch,
        signals: SignalDelivery::new(),
        signaled_frame: None,
    }));

    process_group
        .upgrade()
        .unwrap()
        .lock()
        .add(Arc::downgrade(&child));
    parent.children.push(child.clone());
    process_table.insert(pid, child.clone());
    SCHEDULER.lock().enqueue(pid);
    Ok(child)
}

use crate::{arch::SpinLock, process::Process};
use crate::{arch::SyscallFrame, result::Result};
use alloc::sync::Arc;
use alloc::vec::Vec;
use crossbeam::atomic::AtomicCell;

use super::{alloc_pid, ProcessState, PROCESSES, SCHEDULER};

/// Creates a new process. The calling process (`self`) will be the parent
/// process of the created process. Returns the created child process.
pub fn fork(parent: &Arc<Process>, parent_frame: &SyscallFrame) -> Result<Arc<Process>> {
    let arch = parent.arch.lock().fork(parent_frame)?;
    let vm = parent.vm.as_ref().unwrap().lock().fork()?;
    let opened_files = parent.opened_files.lock().fork();

    let child = Arc::new(Process {
        pid: alloc_pid()?,
        state: AtomicCell::new(ProcessState::Runnable),
        parent: Some(Arc::downgrade(parent)),
        children: SpinLock::new(Vec::new()),
        vm: Some(Arc::new(SpinLock::new(vm))),
        opened_files: Arc::new(SpinLock::new(opened_files)),
        root_fs: parent.root_fs.clone(),
        arch: SpinLock::new(arch),
    });

    parent.children.lock().push(child.clone());
    PROCESSES.lock().insert(child.pid, child.clone());
    SCHEDULER.lock().enqueue(child.clone());
    Ok(child)
}

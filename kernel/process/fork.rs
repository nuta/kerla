use crate::{
    arch::SpinLock,
    process::{Process, WaitQueue},
};
use crate::{arch::SyscallFrame, result::Result};
use alloc::sync::Arc;

use super::{alloc_pid, MutableFields, PROCESSES, SCHEDULER};

/// Creates a new process. The calling process (`self`) will be the parent
/// process of the created process. Returns the created child process.
pub fn fork(parent: &Arc<Process>, parent_frame: &SyscallFrame) -> Result<Arc<Process>> {
    let inner = parent.lock();
    let arch = inner.arch.fork(parent_frame)?;
    let vm = parent.vm.as_ref().unwrap().lock().fork()?;
    let opened_files = parent.opened_files.lock().fork();

    let child = Arc::new(Process {
        pid: alloc_pid()?,
        parent: Some(Arc::downgrade(parent)),
        vm: Some(Arc::new(SpinLock::new(vm))),
        opened_files: Arc::new(SpinLock::new(opened_files)),
        root_fs: parent.root_fs.clone(),
        wait_queue: WaitQueue::new(),
        inner: SpinLock::new(MutableFields {
            arch,
            state: super::ProcessState::Runnable,
            resumed_by: None,
            working_dir: inner.working_dir.clone(),
        }),
    });

    PROCESSES.lock().insert(child.pid, child.clone());
    SCHEDULER.lock().enqueue(child.clone());
    Ok(child)
}

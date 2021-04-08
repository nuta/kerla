use core::sync::atomic::{AtomicI32, Ordering};

use super::*;
use crate::{
    arch::{self, SpinLock},
    boot::INITIAL_ROOT_FS,
    fs::inode::{FileLike, INode},
    fs::{mount::RootFs, opened_file},
    mm::vm::Vm,
    process::execve,
    result::{Errno, Result},
};

use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};

use arch::SpinLockGuard;

use opened_file::OpenedFileTable;

pub static PROCESSES: SpinLock<BTreeMap<PId, Arc<Process>>> = SpinLock::new(BTreeMap::new());

pub fn alloc_pid() -> Result<PId> {
    static NEXT_PID: AtomicI32 = AtomicI32::new(2);

    let last_pid = NEXT_PID.load(Ordering::SeqCst);
    let processes = PROCESSES.lock();
    loop {
        // Note: `fetch_add` may wrap around.
        let pid = NEXT_PID.fetch_add(1, Ordering::SeqCst);
        if pid <= 1 {
            continue;
        }

        if !processes.contains_key(&PId::new(pid)) {
            return Ok(PId::new(pid));
        }

        if pid == last_pid {
            return Err(Errno::EAGAIN.into());
        }
    }
}

pub fn get_process_by_pid(pid: PId) -> Option<Arc<Process>> {
    PROCESSES.lock().get(&pid).cloned()
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PId(i32);

impl PId {
    pub const fn new(pid: i32) -> PId {
        PId(pid)
    }

    pub const fn as_i32(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProcessState {
    Runnable,
    Sleeping,
    WaitForAnyChild,
}

/// Mutable fields in the process struct.
pub struct MutableFields {
    pub arch: arch::Thread,
    pub state: ProcessState,
    pub resumed_by: Option<PId>,
}

/// The process control block.
pub struct Process {
    pub pid: PId,
    pub parent: Option<Weak<Process>>,
    pub vm: Option<Arc<SpinLock<Vm>>>,
    pub opened_files: Arc<SpinLock<OpenedFileTable>>,
    pub root_fs: Arc<SpinLock<RootFs>>,
    pub wait_queue: WaitQueue,
    pub(super) inner: SpinLock<MutableFields>,
}

impl Process {
    /*
    pub fn new_kthread(ip: VAddr) -> Result<Arc<Process>> {
        let stack_bottom = alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE, AllocPageFlags::KERNEL)
            .into_error_with_message(Errno::ENOMEM, "failed to allocate kernel stack")?;
        let sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);
        let process = Arc::new(Process {
            inner: SpinLock::new(MutableFields {
                arch: arch::Thread::new_kthread(ip, sp),
                state: ProcessState::Runnable,
            }),
            vm: None,
            pid: alloc_pid().into_error_with_message(Errno::EAGAIN, "failed to allocate PID")?,
            opened_files: Arc::new(SpinLock::new(OpenedFileTable::new())),
        });

        SCHEDULER.lock().enqueue(process.clone());
        Ok(process)
    }
    */

    pub fn new_idle_thread() -> Result<Arc<Process>> {
        Ok(Arc::new(Process {
            inner: SpinLock::new(MutableFields {
                arch: arch::Thread::new_idle_thread(),
                state: ProcessState::Runnable,
                resumed_by: None,
            }),
            parent: None,
            vm: None,
            pid: PId::new(0),
            root_fs: INITIAL_ROOT_FS.clone(),
            wait_queue: WaitQueue::new(),
            opened_files: Arc::new(SpinLock::new(OpenedFileTable::new())),
        }))
    }

    pub fn new_init_process(
        root_fs: Arc<SpinLock<RootFs>>,
        executable: Arc<dyn FileLike>,
        console: INode,
        argv: &[&[u8]],
    ) -> Result<Arc<Process>> {
        assert!(matches!(console, INode::FileLike(_)));

        let mut opened_files = OpenedFileTable::new();
        // Open stdin.
        opened_files.open_with_fixed_fd(
            Fd::new(0),
            Arc::new(SpinLock::new(OpenedFile::new(
                console.clone(),
                OpenFlags::O_RDONLY.into(),
                0,
            ))),
        )?;
        // Open stdout.
        opened_files.open_with_fixed_fd(
            Fd::new(1),
            Arc::new(SpinLock::new(OpenedFile::new(
                console.clone(),
                OpenFlags::O_WRONLY.into(),
                0,
            ))),
        )?;
        // Open stderr.
        opened_files.open_with_fixed_fd(
            Fd::new(2),
            Arc::new(SpinLock::new(OpenedFile::new(
                console,
                OpenFlags::O_WRONLY.into(),
                0,
            ))),
        )?;

        let process = execve(
            None,
            PId::new(1),
            executable,
            argv,
            &[],
            root_fs,
            Arc::new(SpinLock::new(opened_files)),
        )?;

        PROCESSES.lock().insert(process.pid, process.clone());
        Ok(process)
    }

    pub fn exit(&self) {
        if let Some(parent) = self.parent.as_ref() {
            if let Some(parent) = parent.upgrade() {
                let mut lock = parent.lock();
                // FIXME: What if the child exists before the parent enters the
                //        wait state?
                if ProcessState::WaitForAnyChild == lock.state {
                    // FIXME: Cleanup.
                    lock.state = ProcessState::Runnable;
                    lock.resumed_by = Some(self.pid);
                    drop(lock);
                    SCHEDULER.lock().enqueue(parent);
                }
            }
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, MutableFields> {
        self.inner.lock()
    }

    pub fn vm(&self) -> SpinLockGuard<'_, Vm> {
        self.vm.as_ref().expect("not a user process").lock()
    }
}

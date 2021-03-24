use super::*;
use crate::{
    arch::{self, SpinLock},
    boot::INITIAL_ROOT_FS,
    fs::inode::{FileLike, INode},
    fs::{mount::RootFs, opened_file},
    mm::vm::Vm,
    result::Result,
};

use alloc::sync::Arc;

use arch::SpinLockGuard;

use opened_file::OpenedFileTable;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PId(i32);

impl PId {
    pub const fn new(pid: i32) -> PId {
        PId(pid)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProcessState {
    Runnable,
    Sleeping,
}

/// Mutable fields in the process struct.
pub struct MutableFields {
    pub arch: arch::Thread,
    pub state: ProcessState,
}

/// The process control block.
pub struct Process {
    pub pid: PId,
    pub vm: Option<Arc<SpinLock<Vm>>>,
    pub opened_files: Arc<SpinLock<OpenedFileTable>>,
    pub root_fs: Arc<SpinLock<RootFs>>,
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
            }),
            vm: None,
            pid: PId::new(0),
            root_fs: INITIAL_ROOT_FS.clone(),
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
            Arc::new(OpenedFile::new(console.clone(), OpenMode::O_RDONLY, 0)),
        )?;
        // Open stdout.
        opened_files.open_with_fixed_fd(
            Fd::new(1),
            Arc::new(OpenedFile::new(console.clone(), OpenMode::O_WRONLY, 0)),
        )?;
        // Open stderr.
        opened_files.open_with_fixed_fd(
            Fd::new(2),
            Arc::new(OpenedFile::new(console, OpenMode::O_WRONLY, 0)),
        )?;

        execve(
            PId::new(1),
            executable,
            argv,
            &[],
            root_fs,
            Arc::new(SpinLock::new(opened_files)),
        )
    }

    pub fn lock(&self) -> SpinLockGuard<'_, MutableFields> {
        self.inner.lock()
    }

    pub fn vm(&self) -> SpinLockGuard<'_, Vm> {
        self.vm.as_ref().expect("not a user process").lock()
    }
}

use core::sync::atomic::{AtomicI32, Ordering};

use super::*;
use crate::{
    arch::{self, SpinLock},
    boot::INITIAL_ROOT_FS,
    ctypes::*,
    fs::{mount::RootFs, opened_file},
    mm::vm::Vm,
    process::execve,
    result::{Errno, Result},
};

use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use arch::SpinLockGuard;

use crossbeam::atomic::AtomicCell;
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
    ExitedWith(c_int),
}

/// The process control block.
pub struct Process {
    pub arch: SpinLock<arch::Thread>,
    pub pid: PId,
    pub(super) state: AtomicCell<ProcessState>,
    pub parent: Option<Weak<Process>>,
    pub children: SpinLock<Vec<Arc<Process>>>,
    pub vm: Option<Arc<SpinLock<Vm>>>,
    pub opened_files: Arc<SpinLock<OpenedFileTable>>,
    pub root_fs: Arc<SpinLock<RootFs>>,
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
            arch: SpinLock::new(arch::Thread::new_idle_thread()),
            state: AtomicCell::new(ProcessState::Runnable),
            parent: None,
            children: SpinLock::new(Vec::new()),
            vm: None,
            pid: PId::new(0),
            root_fs: INITIAL_ROOT_FS.clone(),
            opened_files: Arc::new(SpinLock::new(OpenedFileTable::new())),
        }))
    }

    pub fn new_init_process(
        root_fs: Arc<SpinLock<RootFs>>,
        executable_path: Arc<PathComponent>,
        console: Arc<PathComponent>,
        argv: &[&[u8]],
    ) -> Result<()> {
        assert!(console.inode.is_file());

        let mut opened_files = OpenedFileTable::new();
        // Open stdin.
        opened_files.open_with_fixed_fd(
            Fd::new(0),
            Arc::new(SpinLock::new(OpenedFile::new(
                console.clone(),
                OpenFlags::O_RDONLY.into(),
                0,
            ))),
            OpenOptions::empty(),
        )?;
        // Open stdout.
        opened_files.open_with_fixed_fd(
            Fd::new(1),
            Arc::new(SpinLock::new(OpenedFile::new(
                console.clone(),
                OpenFlags::O_WRONLY.into(),
                0,
            ))),
            OpenOptions::empty(),
        )?;
        // Open stderr.
        opened_files.open_with_fixed_fd(
            Fd::new(2),
            Arc::new(SpinLock::new(OpenedFile::new(
                console,
                OpenFlags::O_WRONLY.into(),
                0,
            ))),
            OpenOptions::empty(),
        )?;

        let process = execve(
            None,
            PId::new(1),
            executable_path,
            argv,
            &[],
            root_fs,
            Arc::new(SpinLock::new(opened_files)),
        )?;

        PROCESSES.lock().insert(process.pid, process.clone());
        Ok(())
    }

    pub fn state(&self) -> ProcessState {
        self.state.load()
    }

    pub fn set_state(self: &Arc<Process>, new_state: ProcessState) {
        let scheduler = SCHEDULER.lock();
        let old_state = self.state.swap(new_state);
        if old_state != ProcessState::Runnable && new_state == ProcessState::Runnable {
            scheduler.enqueue(self.clone());
        } else {
            scheduler.remove(self);
        }
    }

    pub fn resume(self: &Arc<Process>) {
        self.set_state(ProcessState::Runnable);
    }

    pub fn exit(self: &Arc<Process>, status: c_int) -> ! {
        self.set_state(ProcessState::ExitedWith(status));
        PROCESSES.lock().remove(&self.pid);
        JOIN_WAIT_QUEUE.wake_all();
        switch();
        unreachable!();
    }

    pub fn vm(&self) -> SpinLockGuard<'_, Vm> {
        self.vm.as_ref().expect("not a user process").lock()
    }
}

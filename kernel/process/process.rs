use core::sync::atomic::{AtomicI32, Ordering};

use super::{
    signal::{Signal, SignalDelivery},
    *,
};
use crate::{
    arch::{self, restore_signaled_stack, setup_signal_handler_stack, SpinLock, SyscallFrame},
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
    pub signals: SpinLock<SignalDelivery>,
    pub syscall_frame: AtomicCell<Option<SyscallFrame>>,
    pub signaled_frame: AtomicCell<Option<SyscallFrame>>,
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
            signals: SpinLock::new(SignalDelivery::new()),
            syscall_frame: AtomicCell::new(None),
            signaled_frame: AtomicCell::new(None),
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

    /// This function may not return if `self` is current process and it has
    /// pending signals. **Ensure no locks are held!**
    pub fn set_state(self: &Arc<Process>, new_state: ProcessState) {
        let scheduler = SCHEDULER.lock();
        let old_state = self.state.swap(new_state);
        match new_state {
            ProcessState::Runnable => {
                if old_state != ProcessState::Runnable {
                    scheduler.enqueue(self.clone());
                }
            }
            ProcessState::Sleeping => {
                scheduler.remove(self);
                drop(scheduler);
                self.try_delivering_signal();
            }
            ProcessState::ExitedWith(_) => {
                scheduler.remove(self);
            }
        }
    }

    pub fn resume(self: &Arc<Process>) {
        self.set_state(ProcessState::Runnable);
    }

    pub fn exit(self: &Arc<Process>, status: c_int) -> ! {
        if self.pid == PId::new(1) {
            panic!("init (pid=0) tried to exit")
        }

        self.set_state(ProcessState::ExitedWith(status));
        if let Some(parent) = self.parent.as_ref() {
            if let Some(parent) = parent.upgrade() {
                parent.signal(Signal::SIGCHLD);
            }
        }

        PROCESSES.lock().remove(&self.pid);
        JOIN_WAIT_QUEUE.wake_all();
        switch();
        unreachable!();
    }

    pub fn vm(&self) -> SpinLockGuard<'_, Vm> {
        self.vm.as_ref().expect("not a user process").lock()
    }

    /// This function may not return if `self` is current process and it has
    /// pending signals. **Ensure no locks are held!**
    pub fn signal(self: &Arc<Process>, signal: Signal) {
        self.signals.lock().signal(signal);
        self.try_delivering_signal();
    }

    /// This function may not return if `self` is current process and it has
    /// pending signals. **Ensure no locks are held!**
    pub fn try_delivering_signal(self: &Arc<Process>) {
        if self.state.load() != ProcessState::Sleeping {
            return;
        }

        // TODO: sigmask

        let pending_signal = { self.signals.lock().pop_pending() };
        if let Some((signal, sigaction)) = pending_signal {
            match sigaction {
                signal::SigAction::Ignore => {}
                signal::SigAction::Handler { handler } => {
                    if let Some(frame) = self.syscall_frame.load() {
                        trace!("delivering {:?} to {:?}", signal, self.pid,);
                        self.signaled_frame.store(Some(frame));

                        // This function may not return if `self` is current process and it has
                        // pending signals.
                        setup_signal_handler_stack(
                            &self.arch,
                            // The vm is only None if `self` is a kernel thread. It's safe to
                            // assume it's always Some.
                            &self.vm.as_ref().unwrap(),
                            &frame,
                            signal,
                            handler,
                            self.pid == current_process().pid,
                        )
                        .unwrap();

                        self.set_state(ProcessState::Runnable);
                    }
                }
            }
        }
    }

    pub fn restore_signaled_user_stack(&self, current_frame: &mut SyscallFrame) {
        // TODO: Kill the process if syscall_frame is empty, i.e., the user
        //       intentionally called sigreturn(2).

        if let Some(signaled_frame) = self.signaled_frame.load() {
            restore_signaled_stack(current_frame, &signaled_frame);
            self.signaled_frame.store(None);
        }
    }
}

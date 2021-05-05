use core::sync::atomic::{AtomicI32, Ordering};

use super::{
    signal::{Signal, SignalDelivery},
    *,
};
use crate::{
    arch::{self, SpinLock, SyscallFrame},
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

use opened_file::OpenedFileTable;

pub static PROCESSES: SpinLock<BTreeMap<PId, Arc<SpinLock<Process>>>> =
    SpinLock::new(BTreeMap::new());

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
    pub arch: arch::Thread,
    pub pid: PId,
    pub state: ProcessState,
    pub parent: Option<Weak<SpinLock<Process>>>,
    pub children: Vec<Arc<SpinLock<Process>>>,
    pub vm: Option<Arc<SpinLock<Vm>>>,
    pub opened_files: Arc<SpinLock<OpenedFileTable>>,
    pub root_fs: Arc<SpinLock<RootFs>>,
    pub signals: SignalDelivery,
    pub signaled_frame: Option<SyscallFrame>,
}

impl Process {
    /*
    pub fn new_kthread(ip: VAddr) -> Result<Arc<SpinLock<Process>>> {
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

    pub fn new_idle_thread() -> Result<Arc<SpinLock<Process>>> {
        Ok(Arc::new(SpinLock::new(Process {
            arch: arch::Thread::new_idle_thread(),
            state: ProcessState::Runnable,
            parent: None,
            children: Vec::new(),
            vm: None,
            pid: PId::new(0),
            root_fs: INITIAL_ROOT_FS.clone(),
            opened_files: Arc::new(SpinLock::new(OpenedFileTable::new())),
            signals: SignalDelivery::new(),
            signaled_frame: None,
        })))
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

        PROCESSES.lock().insert(process.lock().pid, process.clone());
        Ok(())
    }

    pub fn pid(&self) -> PId {
        self.pid
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn set_state(&mut self, new_state: ProcessState) {
        let scheduler = SCHEDULER.lock();
        let old_state = self.state;
        self.state = new_state;
        match new_state {
            ProcessState::Runnable => {
                if old_state != ProcessState::Runnable {
                    scheduler.enqueue(self.pid);
                }
            }
            ProcessState::Sleeping => {
                scheduler.remove(self.pid);
            }
            ProcessState::ExitedWith(_) => {
                scheduler.remove(self.pid);
            }
        }
    }

    pub fn get_opened_file_by_fd(&self, fd: Fd) -> Result<Arc<SpinLock<OpenedFile>>> {
        Ok(self.opened_files.lock().get(fd)?.clone())
    }

    pub fn exit(mut proc: SpinLockGuard<'_, Process>, status: c_int) -> ! {
        if proc.pid == PId::new(1) {
            panic!("init (pid=0) tried to exit")
        }

        proc.set_state(ProcessState::ExitedWith(status));
        if let Some(parent) = proc.parent.as_ref() {
            if let Some(parent) = parent.upgrade() {
                parent.lock().signal(Signal::SIGCHLD);
            }
        }

        PROCESSES.lock().remove(&proc.pid);
        JOIN_WAIT_QUEUE.wake_all();
        drop(proc);
        switch();
        unreachable!();
    }

    pub fn vm(&self) -> SpinLockGuard<'_, Vm> {
        self.vm.as_ref().expect("not a user process").lock()
    }

    pub fn signal(&mut self, signal: Signal) {
        self.signals.signal(signal);
    }

    pub fn is_signal_pending(&self) -> bool {
        self.signals.is_pending()
    }

    pub fn try_delivering_signal(&mut self, frame: &mut SyscallFrame) -> Result<()> {
        // TODO: sigmask

        if let Some((signal, sigaction)) = self.signals.pop_pending() {
            match sigaction {
                signal::SigAction::Ignore => {}
                signal::SigAction::Handler { handler } => {
                    trace!("delivering {:?} to {:?}", signal, self.pid,);
                    self.signaled_frame = Some(*frame);
                    unsafe {
                        self.arch.setup_signal_stack(frame, signal, handler)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn restore_signaled_user_stack(&mut self, current_frame: &mut SyscallFrame) {
        if let Some(signaled_frame) = self.signaled_frame.take() {
            self.arch
                .setup_sigreturn_stack(current_frame, &signaled_frame);
        } else {
            // The user intentionally called sigreturn(2) while it is not signaled.
            kill_current_process();
        }
    }

    pub fn execved(mut held_lock: SpinLockGuard<'_, Process>) -> ! {
        held_lock.state = ProcessState::Sleeping;
        drop(held_lock);
        switch();
        unreachable!();
    }
}

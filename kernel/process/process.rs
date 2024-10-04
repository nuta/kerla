use crate::{
    arch::{self, USER_STACK_TOP},
    ctypes::*,
    fs::{
        devfs::SERIAL_TTY,
        inode::FileLike,
        mount::RootFs,
        opened_file::{Fd, OpenFlags, OpenOptions, OpenedFile, OpenedFileTable, PathComponent},
        path::Path,
    },
    mm::vm::{Vm, VmAreaType},
    prelude::*,
    process::{
        cmdline::Cmdline,
        current_process,
        elf::{Elf, ProgramHeader},
        init_stack::{estimate_user_init_stack_size, init_user_stack, Auxv},
        process_group::{PgId, ProcessGroup},
        signal::{SigAction, SigSet, Signal, SignalDelivery, SignalMask, SIGCHLD, SIGKILL},
        switch, UserVAddr, JOIN_WAIT_QUEUE, SCHEDULER,
    },
    random::read_secure_random,
    result::Errno,
    INITIAL_ROOT_FS,
};
use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use atomic_refcell::{AtomicRef, AtomicRefCell};
use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};
use core::{cmp::max, sync::atomic::AtomicUsize};
use crossbeam::atomic::AtomicCell;
use goblin::elf64::program_header::PT_LOAD;
use kerla_runtime::{
    arch::{PtRegs, PAGE_SIZE},
    page_allocator::{alloc_pages, AllocPageFlags},
    spinlock::{SpinLock, SpinLockGuard},
};
use kerla_utils::alignment::align_up;

type ProcessTable = BTreeMap<PId, Arc<Process>>;

/// The process table. All processes are registered in with its process Id.
pub(super) static PROCESSES: SpinLock<ProcessTable> = SpinLock::new(BTreeMap::new());
pub(super) static EXITED_PROCESSES: SpinLock<Vec<Arc<Process>>> = SpinLock::new(Vec::new());

static FORK_TOTAL: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub struct Stats {
    pub fork_total: usize,
}

pub fn read_process_stats() -> Stats {
    Stats {
        fork_total: FORK_TOTAL.load(Ordering::SeqCst),
    }
}

/// Returns an unused PID. Note that this function does not reserve the PID:
/// keep the process table locked until you insert the process into the table!
pub(super) fn alloc_pid(table: &mut ProcessTable) -> Result<PId> {
    static NEXT_PID: AtomicI32 = AtomicI32::new(2);

    let last_pid = NEXT_PID.load(Ordering::SeqCst);
    loop {
        // Note: `fetch_add` may wrap around.
        let pid = NEXT_PID.fetch_add(1, Ordering::SeqCst);
        if pid <= 1 {
            continue;
        }

        if !table.contains_key(&PId::new(pid)) {
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

/// Process states.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProcessState {
    /// The process is runnable.
    Runnable,
    /// The process is sleeping. It can be resumed by signals.
    BlockedSignalable,
    /// The process has exited.
    ExitedWith(c_int),
}

/// The process control block.
pub struct Process {
    arch: arch::Process,
    is_idle: bool,
    process_group: AtomicRefCell<Weak<SpinLock<ProcessGroup>>>,
    pid: PId,
    state: AtomicCell<ProcessState>,
    parent: Weak<Process>,
    cmdline: AtomicRefCell<Cmdline>,
    children: SpinLock<Vec<Arc<Process>>>,
    vm: AtomicRefCell<Option<Arc<SpinLock<Vm>>>>,
    opened_files: Arc<SpinLock<OpenedFileTable>>,
    root_fs: Arc<SpinLock<RootFs>>,
    signals: Arc<SpinLock<SignalDelivery>>,
    signaled_frame: AtomicCell<Option<PtRegs>>,
    sigset: SpinLock<SigSet>,
}

impl Process {
    /// Creates a per-CPU idle thread.
    ///
    /// An idle thread is a special type of kernel threads which is executed
    /// only if there're no other runnable processes.
    pub fn new_idle_thread() -> Result<Arc<Process>> {
        let process_group = ProcessGroup::new(PgId::new(0));
        let proc = Arc::new(Process {
            is_idle: true,
            process_group: AtomicRefCell::new(Arc::downgrade(&process_group)),
            arch: arch::Process::new_idle_thread(),
            state: AtomicCell::new(ProcessState::Runnable),
            parent: Weak::new(),
            cmdline: AtomicRefCell::new(Cmdline::new()),
            children: SpinLock::new(Vec::new()),
            vm: AtomicRefCell::new(None),
            pid: PId::new(0),
            root_fs: INITIAL_ROOT_FS.clone(),
            opened_files: Arc::new(SpinLock::new(OpenedFileTable::new())),
            signals: Arc::new(SpinLock::new(SignalDelivery::new())),
            signaled_frame: AtomicCell::new(None),
            sigset: SpinLock::new(SigSet::ZERO),
        });

        process_group.lock().add(Arc::downgrade(&proc));
        Ok(proc)
    }

    /// Creates the initial process (PID=1).
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
            Arc::new(OpenedFile::new(
                console.clone(),
                OpenFlags::O_RDONLY.into(),
                0,
            )),
            OpenOptions::empty(),
        )?;
        // Open stdout.
        opened_files.open_with_fixed_fd(
            Fd::new(1),
            Arc::new(OpenedFile::new(
                console.clone(),
                OpenFlags::O_WRONLY.into(),
                0,
            )),
            OpenOptions::empty(),
        )?;
        // Open stderr.
        opened_files.open_with_fixed_fd(
            Fd::new(2),
            Arc::new(OpenedFile::new(console, OpenFlags::O_WRONLY.into(), 0)),
            OpenOptions::empty(),
        )?;

        let entry = setup_userspace(executable_path, argv, &[], &root_fs)?;
        let pid = PId::new(1);
        let process_group = ProcessGroup::new(PgId::new(1));
        let process = Arc::new(Process {
            is_idle: false,
            process_group: AtomicRefCell::new(Arc::downgrade(&process_group)),
            pid,
            parent: Weak::new(),
            children: SpinLock::new(Vec::new()),
            state: AtomicCell::new(ProcessState::Runnable),
            cmdline: AtomicRefCell::new(Cmdline::from_argv(argv)),
            arch: arch::Process::new_user_thread(entry.ip, entry.user_sp),
            vm: AtomicRefCell::new(Some(Arc::new(SpinLock::new(entry.vm)))),
            opened_files: Arc::new(SpinLock::new(opened_files)),
            root_fs,
            signals: Arc::new(SpinLock::new(SignalDelivery::new())),
            signaled_frame: AtomicCell::new(None),
            sigset: SpinLock::new(SigSet::ZERO),
        });

        process_group.lock().add(Arc::downgrade(&process));
        PROCESSES.lock().insert(pid, process);
        SCHEDULER.lock().enqueue(pid);

        SERIAL_TTY.set_foreground_process_group(Arc::downgrade(&process_group));
        Ok(())
    }

    /// Returns the process with the given process ID.
    pub fn find_by_pid(pid: PId) -> Option<Arc<Process>> {
        PROCESSES.lock().get(&pid).cloned()
    }

    /// Returns true if the process is a idle kernel thread.
    pub fn is_idle(&self) -> bool {
        self.is_idle
    }

    /// The process ID.
    pub fn pid(&self) -> PId {
        self.pid
    }

    /// The thread ID.
    pub fn tid(&self) -> PId {
        // In a single-threaded process, the thread ID is equal to the process ID (PID).
        // https://man7.org/linux/man-pages/man2/gettid.2.html
        self.pid
    }

    /// The arch-specific information.
    pub fn arch(&self) -> &arch::Process {
        &self.arch
    }

    /// The process parent.
    fn parent(&self) -> Option<Arc<Process>> {
        self.parent.upgrade().as_ref().cloned()
    }

    /// The ID of process being parent of this process.
    pub fn ppid(&self) -> PId {
        if let Some(parent) = self.parent() {
            parent.pid()
        } else {
            PId::new(0)
        }
    }

    pub fn cmdline(&self) -> AtomicRef<'_, Cmdline> {
        self.cmdline.borrow()
    }

    /// Its child processes.
    pub fn children(&self) -> SpinLockGuard<'_, Vec<Arc<Process>>> {
        self.children.lock()
    }

    /// The process's path resolution info.
    pub fn root_fs(&self) -> &Arc<SpinLock<RootFs>> {
        &self.root_fs
    }

    /// The ppened files table.
    pub fn opened_files(&self) -> &Arc<SpinLock<OpenedFileTable>> {
        &self.opened_files
    }

    /// The virtual memory space. It's `None` if the process is a kernel thread.
    pub fn vm(&self) -> AtomicRef<'_, Option<Arc<SpinLock<Vm>>>> {
        self.vm.borrow()
    }

    /// Signals.
    pub fn signals(&self) -> &SpinLock<SignalDelivery> {
        &self.signals
    }

    /// Changes the process group.
    pub fn set_process_group(&self, pg: Weak<SpinLock<ProcessGroup>>) {
        *self.process_group.borrow_mut() = pg;
    }

    /// The current process group.
    pub fn process_group(&self) -> Arc<SpinLock<ProcessGroup>> {
        self.process_group.borrow().upgrade().unwrap()
    }

    /// Returns true if the process belongs to the process group `pg`.
    pub fn belongs_to_process_group(&self, pg: &Weak<SpinLock<ProcessGroup>>) -> bool {
        Weak::ptr_eq(&self.process_group.borrow(), pg)
    }

    /// The current process state.
    pub fn state(&self) -> ProcessState {
        self.state.load()
    }

    /// Updates the process state.
    pub fn set_state(&self, new_state: ProcessState) {
        let scheduler = SCHEDULER.lock();
        self.state.store(new_state);
        match new_state {
            ProcessState::Runnable => {}
            ProcessState::BlockedSignalable | ProcessState::ExitedWith(_) => {
                scheduler.remove(self.pid);
            }
        }
    }

    /// Resumes a process.
    pub fn resume(&self) {
        let old_state = self.state.swap(ProcessState::Runnable);

        debug_assert!(!matches!(old_state, ProcessState::ExitedWith(_)));

        if old_state == ProcessState::Runnable {
            return;
        }

        SCHEDULER.lock().enqueue(self.pid);
    }

    /// Searches the opned file table by the file descriptor.
    pub fn get_opened_file_by_fd(&self, fd: Fd) -> Result<Arc<OpenedFile>> {
        Ok(self.opened_files.lock().get(fd)?.clone())
    }

    /// Terminates the **current** process.
    pub fn exit(status: c_int) -> ! {
        let current = current_process();
        if current.pid == PId::new(1) {
            panic!("init (pid=0) tried to exit")
        }

        current.set_state(ProcessState::ExitedWith(status));
        if let Some(parent) = current.parent.upgrade() {
            if parent.signals().lock().get_action(SIGCHLD) == SigAction::Ignore {
                // If the parent process is not waiting for a child,
                // remove the child from its list.
                parent.children().retain(|p| p.pid() != current.pid);

                // Keep the reference because we're using its kernel stack. Postpone
                // freeing the stack until we move from the current thread.
                EXITED_PROCESSES.lock().push(current.clone());
            } else {
                parent.send_signal(SIGCHLD)
            }
        }

        // Close opened files here instead of in Drop::drop because `proc` is
        // not dropped until it's joined by the parent process. Drop them to
        // make pipes closed.
        current.opened_files.lock().close_all();

        PROCESSES.lock().remove(&current.pid);
        JOIN_WAIT_QUEUE.wake_all();
        switch();
        unreachable!();
    }

    /// Terminates the **current** thread and other threads belonging to the same thread group.
    pub fn exit_group(status: c_int) -> ! {
        // TODO: Kill other threads belonging to the same thread group.
        Process::exit(status)
    }

    /// Terminates the **current** process by a signal.
    pub fn exit_by_signal(_signal: Signal) -> ! {
        Process::exit(1 /* FIXME: how should we compute the exit status? */);
    }

    /// Sends a signal.
    pub fn send_signal(&self, signal: Signal) {
        self.signals.lock().signal(signal);
        self.resume();
    }

    /// Returns `true` if there's a pending signal.
    pub fn has_pending_signals(&self) -> bool {
        self.signals.lock().is_pending()
    }

    /// Sets signal mask.
    pub fn set_signal_mask(
        &self,
        how: SignalMask,
        set: Option<UserVAddr>,
        oldset: Option<UserVAddr>,
        _length: usize,
    ) -> Result<()> {
        let mut sigset = self.sigset.lock();

        if let Some(old) = oldset {
            old.write_bytes(sigset.as_raw_slice())?;
        }

        if let Some(new) = set {
            let new_set = new.read::<[u8; 128]>()?;
            let new_set = SigSet::new(new_set);
            match how {
                SignalMask::Block => *sigset |= new_set,
                SignalMask::Unblock => *sigset &= !new_set,
                SignalMask::Set => *sigset = new_set,
            }
        }

        Ok(())
    }

    /// Tries to delivering a pending signal to the current process.
    ///
    /// If there's a pending signal, it may modify `frame` (e.g. user return
    /// address and stack pointer) to call the registered user's signal handler.
    pub fn try_delivering_signal(frame: &mut PtRegs) -> Result<()> {
        let current = current_process();
        if let Some((signal, sigaction)) = current.signals.lock().pop_pending() {
            let sigset = current.sigset.lock();
            if !sigset.get(signal as usize).as_deref().unwrap_or(&true) {
                match sigaction {
                    SigAction::Ignore => {}
                    SigAction::Terminate => {
                        trace!("terminating {:?} by {:?}", current.pid, signal,);
                        Process::exit(1 /* FIXME: */);
                    }
                    SigAction::Handler { handler } => {
                        trace!("delivering {:?} to {:?}", signal, current.pid,);
                        current.signaled_frame.store(Some(*frame));
                        unsafe {
                            current.arch.setup_signal_stack(frame, signal, handler)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// So-called `sigreturn`: restores the user context when the signal is
    /// delivered to a signal handler.
    pub fn restore_signaled_user_stack(current: &Arc<Process>, current_frame: &mut PtRegs) {
        if let Some(signaled_frame) = current.signaled_frame.swap(None) {
            current
                .arch
                .setup_sigreturn_stack(current_frame, &signaled_frame);
        } else {
            // The user intentionally called sigreturn(2) while it is not signaled.
            // TODO: Should we ignore instead of the killing the process?
            Process::exit_by_signal(SIGKILL);
        }
    }

    /// Creates a new virtual memory space, loads the executable, and overwrites
    /// the **current** process.
    ///
    /// It modifies `frame` to start from the new executable's entry point with
    /// new stack (ie. argv and envp) when the system call handler returns into
    /// the userspace.
    pub fn execve(
        frame: &mut PtRegs,
        executable_path: Arc<PathComponent>,
        argv: &[&[u8]],
        envp: &[&[u8]],
    ) -> Result<()> {
        let current = current_process();
        current.opened_files.lock().close_cloexec_files();
        current.cmdline.borrow_mut().set_by_argv(argv);

        let entry = setup_userspace(executable_path, argv, envp, &current.root_fs)?;

        // FIXME: Should we prevent try_delivering_signal()?
        current.signaled_frame.store(None);

        entry.vm.page_table().switch();
        *current.vm.borrow_mut() = Some(Arc::new(SpinLock::new(entry.vm)));

        current
            .arch
            .setup_execve_stack(frame, entry.ip, entry.user_sp)?;

        Ok(())
    }

    /// Creates a new process. The calling process (`self`) will be the parent
    /// process of the created process. Returns the created child process.
    pub fn fork(parent: &Arc<Process>, parent_frame: &PtRegs) -> Result<Arc<Process>> {
        let parent_weak = Arc::downgrade(parent);
        let mut process_table = PROCESSES.lock();
        let pid = alloc_pid(&mut process_table)?;
        let arch = parent.arch.fork(parent_frame)?;
        let vm = parent.vm().as_ref().unwrap().lock().fork()?;
        let opened_files = parent.opened_files().lock().clone(); // TODO: #88 has to address this
        let process_group = parent.process_group();
        let sig_set = parent.sigset.lock();

        let child = Arc::new(Process {
            is_idle: false,
            process_group: AtomicRefCell::new(Arc::downgrade(&process_group)),
            pid,
            state: AtomicCell::new(ProcessState::Runnable),
            parent: parent_weak,
            cmdline: AtomicRefCell::new(parent.cmdline().clone()),
            children: SpinLock::new(Vec::new()),
            vm: AtomicRefCell::new(Some(Arc::new(SpinLock::new(vm)))),
            opened_files: Arc::new(SpinLock::new(opened_files)),
            root_fs: parent.root_fs().clone(),
            arch,
            signals: Arc::new(SpinLock::new(SignalDelivery::new())), // TODO: #88 has to address this
            signaled_frame: AtomicCell::new(None),
            sigset: SpinLock::new(*sig_set),
        });

        process_group.lock().add(Arc::downgrade(&child));
        parent.children().push(child.clone());
        process_table.insert(pid, child.clone());
        SCHEDULER.lock().enqueue(pid);

        FORK_TOTAL.fetch_add(1, Ordering::Relaxed);
        Ok(child)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        trace!(
            "dropping {:?} (cmdline={})",
            self.pid(),
            self.cmdline().as_str()
        );

        // Since the process's reference count has already reached to zero (that's
        // why the process is being dropped), ProcessGroup::remove_dropped_processes
        // should remove this process from its list.
        self.process_group().lock().remove_dropped_processes();
    }
}

struct UserspaceEntry {
    vm: Vm,
    ip: UserVAddr,
    user_sp: UserVAddr,
}

fn setup_userspace(
    executable_path: Arc<PathComponent>,
    argv: &[&[u8]],
    envp: &[&[u8]],
    root_fs: &Arc<SpinLock<RootFs>>,
) -> Result<UserspaceEntry> {
    do_setup_userspace(executable_path, argv, envp, root_fs, true)
}

fn do_script_binfmt(
    executable_path: &Arc<PathComponent>,
    script_argv: &[&[u8]],
    envp: &[&[u8]],
    root_fs: &Arc<SpinLock<RootFs>>,
    buf: &[u8],
) -> Result<UserspaceEntry> {
    // Set up argv[] with the interpreter and its arguments from the shebang line.
    let mut argv: Vec<&[u8]> = buf[2..buf.iter().position(|&ch| ch == b'\n').unwrap()]
        .split(|&ch| ch == b' ')
        .collect();
    if argv.is_empty() {
        return Err(Errno::EINVAL.into());
    }

    // Push the path to the script file as the first argument to the
    // interpreter.
    let executable_pathbuf = executable_path.resolve_absolute_path();
    argv.push(executable_pathbuf.as_str().as_bytes());

    // Push the original arguments to the script on after the new script
    // invocation (leaving out argv[0] of the previous path of invoking the
    // script.)
    for arg in script_argv.iter().skip(1) {
        argv.push(arg);
    }

    let shebang_path = root_fs.lock().lookup_path(
        Path::new(core::str::from_utf8(argv[0]).map_err(|_| Error::new(Errno::EINVAL))?),
        true,
    )?;

    do_setup_userspace(shebang_path, &argv, envp, root_fs, false)
}

fn do_elf_binfmt(
    executable: &Arc<dyn FileLike>,
    argv: &[&[u8]],
    envp: &[&[u8]],
    file_header_pages: kerla_api::address::PAddr,
    buf: &[u8],
) -> Result<UserspaceEntry> {
    let file_header_top = USER_STACK_TOP;
    let elf = Elf::parse(buf)?;
    let ip = elf.entry()?;

    let mut end_of_image = 0;
    for phdr in elf.program_headers() {
        if phdr.p_type == PT_LOAD {
            end_of_image = max(end_of_image, (phdr.p_vaddr + phdr.p_memsz) as usize);
        }
    }

    let mut random_bytes = [0u8; 16];
    read_secure_random(((&mut random_bytes) as &mut [u8]).into())?;

    // Set up the user stack.
    let auxv = &[
        Auxv::Phdr(
            file_header_top
                .sub(buf.len())
                .add(elf.header().e_phoff as usize),
        ),
        Auxv::Phnum(elf.program_headers().len()),
        Auxv::Phent(size_of::<ProgramHeader>()),
        Auxv::Pagesz(PAGE_SIZE),
        Auxv::Random(random_bytes),
    ];
    const USER_STACK_LEN: usize = 128 * 1024; // TODO: Implement rlimit
    let init_stack_top = file_header_top.sub(buf.len());
    let user_stack_bottom = init_stack_top.sub(USER_STACK_LEN).value();
    let user_heap_bottom = align_up(end_of_image, PAGE_SIZE);
    let init_stack_len = align_up(estimate_user_init_stack_size(argv, envp, auxv), PAGE_SIZE);
    if user_heap_bottom >= user_stack_bottom || init_stack_len >= USER_STACK_LEN {
        return Err(Errno::E2BIG.into());
    }

    let init_stack_pages = alloc_pages(init_stack_len / PAGE_SIZE, AllocPageFlags::KERNEL)?;
    let user_sp = init_user_stack(
        init_stack_top,
        init_stack_pages.as_vaddr().add(init_stack_len),
        init_stack_pages.as_vaddr(),
        argv,
        envp,
        auxv,
    )?;

    let mut vm = Vm::new(
        UserVAddr::new(user_stack_bottom).unwrap(),
        UserVAddr::new(user_heap_bottom).unwrap(),
    )?;
    for i in 0..(buf.len() / PAGE_SIZE) {
        vm.page_table_mut().map_user_page(
            file_header_top.sub(((buf.len() / PAGE_SIZE) - i) * PAGE_SIZE),
            file_header_pages.add(i * PAGE_SIZE),
        );
    }

    for i in 0..(init_stack_len / PAGE_SIZE) {
        vm.page_table_mut().map_user_page(
            init_stack_top.sub(((init_stack_len / PAGE_SIZE) - i) * PAGE_SIZE),
            init_stack_pages.add(i * PAGE_SIZE),
        );
    }

    // Register program headers in the virtual memory space.
    for phdr in elf.program_headers() {
        if phdr.p_type != PT_LOAD {
            continue;
        }

        let area_type = if phdr.p_filesz > 0 {
            VmAreaType::File {
                file: executable.clone(),
                offset: phdr.p_offset as usize,
                file_size: phdr.p_filesz as usize,
            }
        } else {
            VmAreaType::Anonymous
        };

        vm.add_vm_area(
            UserVAddr::new_nonnull(phdr.p_vaddr as usize)?,
            phdr.p_memsz as usize,
            area_type,
        )?;
    }

    Ok(UserspaceEntry { vm, ip, user_sp })
}

/// Creates a new virtual memory space, parses and maps an executable file,
/// and set up the user stack.
fn do_setup_userspace(
    executable_path: Arc<PathComponent>,
    argv: &[&[u8]],
    envp: &[&[u8]],
    root_fs: &Arc<SpinLock<RootFs>>,
    handle_shebang: bool,
) -> Result<UserspaceEntry> {
    // Read the ELF header in the executable file.
    let file_header_len = PAGE_SIZE;
    let file_header_pages = alloc_pages(file_header_len / PAGE_SIZE, AllocPageFlags::KERNEL)?;
    let buf =
        unsafe { core::slice::from_raw_parts_mut(file_header_pages.as_mut_ptr(), file_header_len) };

    let executable = executable_path.inode.as_file()?;
    executable.read(0, buf.into(), &OpenOptions::readwrite())?;

    if handle_shebang && buf.starts_with(b"#!") && buf.contains(&b'\n') {
        return do_script_binfmt(&executable_path, argv, envp, root_fs, buf);
    }

    do_elf_binfmt(executable, argv, envp, file_header_pages, buf)
}

pub fn gc_exited_processes() {
    if current_process().is_idle() {
        // If we're in an idle thread, it's safe to free kernel stacks allocated
        // for other exited processes.
        EXITED_PROCESSES.lock().clear();
    }
}

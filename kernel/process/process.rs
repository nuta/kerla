use super::{
    elf::{Elf, ProgramHeader},
    process_group::{PgId, ProcessGroup},
    signal::{Signal, SignalDelivery},
    *,
};
use crate::{
    arch::{self, SpinLock, SyscallFrame},
    boot::INITIAL_ROOT_FS,
    ctypes::*,
    fs::{
        mount::RootFs,
        opened_file::{OpenOptions, OpenedFileTable},
        path::Path,
    },
    mm::vm::{Vm, VmAreaType},
    random::read_secure_random,
};
use crate::{
    fs::devfs::SERIAL_TTY,
    mm::page_allocator::{alloc_pages, AllocPageFlags},
};
use crate::{prelude::*, process::signal::SIGCHLD};
use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use arch::SpinLockGuard;
use core::sync::atomic::{AtomicI32, Ordering};
use goblin::elf64::program_header::PT_LOAD;

type ProcessTable = BTreeMap<PId, Arc<SpinLock<Process>>>;
pub static PROCESSES: SpinLock<ProcessTable> = SpinLock::new(BTreeMap::new());

/// Returns an unused PID. Note that this function does not reserve the PID:
/// keep the process table locked until you insert the process into the table!
pub fn alloc_pid(table: &mut ProcessTable) -> Result<PId> {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProcessState {
    Runnable,
    Sleeping,
    ExitedWith(c_int),
}

/// The process control block.
pub struct Process {
    pub arch: arch::Thread,
    pub(super) process_group: Weak<SpinLock<ProcessGroup>>,
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
        let process_group = ProcessGroup::new(PgId::new(0));
        let proc = Arc::new(SpinLock::new(Process {
            process_group: Arc::downgrade(&process_group),
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
        }));

        process_group.lock().add(Arc::downgrade(&proc));
        Ok(proc)
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

        let entry = setup_userspace(executable_path, argv, &[], &root_fs)?;
        let pid = PId::new(1);
        let stack_bottom = alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE, AllocPageFlags::KERNEL)?;
        let kernel_sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);
        let process_group = ProcessGroup::new(PgId::new(1));
        let process = Arc::new(SpinLock::new(Process {
            process_group: Arc::downgrade(&process_group),
            pid,
            parent: None,
            children: Vec::new(),
            state: ProcessState::Runnable,
            arch: arch::Thread::new_user_thread(entry.ip, entry.user_sp, kernel_sp),
            vm: Some(Arc::new(SpinLock::new(entry.vm))),
            opened_files: Arc::new(SpinLock::new(opened_files)),
            root_fs,
            signals: SignalDelivery::new(),
            signaled_frame: None,
        }));

        process_group.lock().add(Arc::downgrade(&process));
        PROCESSES.lock().insert(pid, process.clone());
        SCHEDULER.lock().enqueue(pid);

        SERIAL_TTY.set_foreground_process_group(Arc::downgrade(&process_group));
        Ok(())
    }

    pub fn find_by_pid(pid: PId) -> Option<Arc<SpinLock<Process>>> {
        PROCESSES.lock().get(&pid).cloned()
    }

    pub fn pid(&self) -> PId {
        self.pid
    }

    pub fn set_process_group(&mut self, pg: Weak<SpinLock<ProcessGroup>>) {
        self.process_group = pg;
    }

    pub fn process_group(&self) -> Arc<SpinLock<ProcessGroup>> {
        self.process_group.upgrade().unwrap()
    }

    pub fn process_group_weak(&self) -> &Weak<SpinLock<ProcessGroup>> {
        &self.process_group
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
                parent.lock().signal(SIGCHLD);
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

    pub fn try_delivering_signal(
        mut current: SpinLockGuard<'_, Process>,
        frame: &mut SyscallFrame,
    ) -> Result<()> {
        // TODO: sigmask

        if let Some((signal, sigaction)) = current.signals.pop_pending() {
            match sigaction {
                signal::SigAction::Ignore => {}
                signal::SigAction::Terminate => {
                    trace!("terminating {:?} by {:?}", current.pid, signal,);
                    Process::exit(current, 1 /* FIXME: */);
                }
                signal::SigAction::Handler { handler } => {
                    trace!("delivering {:?} to {:?}", signal, current.pid,);
                    current.signaled_frame = Some(*frame);
                    unsafe {
                        current.arch.setup_signal_stack(frame, signal, handler)?;
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

    pub fn execve(
        &mut self,
        frame: &mut SyscallFrame,
        executable_path: Arc<PathComponent>,
        argv: &[&[u8]],
        envp: &[&[u8]],
    ) -> Result<()> {
        self.opened_files.lock().close_cloexec_files();
        let entry = setup_userspace(executable_path, argv, envp, &self.root_fs)?;

        // FIXME: Should we prevent try_delivering_signal()?
        self.signaled_frame = None;

        entry.vm.page_table().switch();
        self.vm = Some(Arc::new(SpinLock::new(entry.vm)));

        self.arch
            .setup_execve_stack(frame, entry.ip, entry.user_sp)?;
        Ok(())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.process_group
            .upgrade()
            .unwrap()
            .lock()
            .remove_dropped_processes();
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

#[allow(clippy::too_many_arguments)]
fn do_setup_userspace(
    executable_path: Arc<PathComponent>,
    argv: &[&[u8]],
    envp: &[&[u8]],
    root_fs: &Arc<SpinLock<RootFs>>,
    handle_shebang: bool,
) -> Result<UserspaceEntry> {
    // Read the ELF header in the executable file.
    let file_header_len = PAGE_SIZE;
    let file_header_top = USER_STACK_TOP;
    let file_header_pages = alloc_pages(file_header_len / PAGE_SIZE, AllocPageFlags::KERNEL)?;
    let buf =
        unsafe { core::slice::from_raw_parts_mut(file_header_pages.as_mut_ptr(), file_header_len) };

    let executable = executable_path.inode.as_file()?;
    executable.read(0, buf.into(), &OpenOptions::readwrite())?;

    if handle_shebang && buf.starts_with(b"#!") && buf.contains(&b'\n') {
        let mut argv: Vec<&[u8]> = buf[2..buf.iter().position(|&ch| ch == b'\n').unwrap()]
            .split(|&ch| ch == b' ')
            .collect();
        if argv.is_empty() {
            return Err(Errno::EINVAL.into());
        }

        let executable_pathbuf = executable_path.resolve_absolute_path();
        argv.push(executable_pathbuf.as_str().as_bytes());

        let shebang_path = root_fs.lock().lookup_path(
            Path::new(core::str::from_utf8(argv[0]).map_err(|_| Error::new(Errno::EINVAL))?),
            true,
        )?;

        return do_setup_userspace(shebang_path, &argv, envp, root_fs, false);
    }

    let elf = Elf::parse(&buf)?;
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
                .sub(file_header_len)?
                .add(elf.header().e_phoff as usize)?,
        ),
        Auxv::Phnum(elf.program_headers().len()),
        Auxv::Phent(size_of::<ProgramHeader>()),
        Auxv::Pagesz(PAGE_SIZE),
        Auxv::Random(random_bytes),
    ];
    const USER_STACK_LEN: usize = 128 * 1024; // TODO: Implement rlimit
    let init_stack_top = file_header_top.sub(file_header_len)?;
    let user_stack_bottom = init_stack_top.sub(USER_STACK_LEN).unwrap().value();
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
        UserVAddr::new_nonnull(user_stack_bottom).unwrap(),
        UserVAddr::new_nonnull(user_heap_bottom).unwrap(),
    )?;
    for i in 0..(file_header_len / PAGE_SIZE) {
        vm.page_table_mut().map_user_page(
            file_header_top
                .sub(((file_header_len / PAGE_SIZE) - i) * PAGE_SIZE)
                .unwrap(),
            file_header_pages.add(i * PAGE_SIZE),
        );
    }

    for i in 0..(init_stack_len / PAGE_SIZE) {
        vm.page_table_mut().map_user_page(
            init_stack_top
                .sub(((init_stack_len / PAGE_SIZE) - i) * PAGE_SIZE)
                .unwrap(),
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

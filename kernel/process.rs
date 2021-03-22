use crate::{
    arch::{self, disable_interrupt, enable_interrupt, is_interrupt_enabled, SpinLock, VAddr},
    elf::Elf,
    fs::initramfs::INITRAM_FS,
    fs::mount::RootFs,
    fs::path::Path,
    fs::{
        inode::{FileLike, INode},
        stat::Stat,
    },
    mm::{
        page_allocator::alloc_pages,
        vm::{Vm, VmAreaType},
    },
    result::{Errno, Error, ErrorExt, Result},
};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::{UserVAddr, KERNEL_STACK_SIZE, PAGE_SIZE, USER_STACK_TOP};
use arrayvec::ArrayVec;
use core::cmp::max;
use core::mem::{self, size_of, size_of_val};
use core::sync::atomic::{AtomicI32, Ordering};
use goblin::elf64::program_header::PT_LOAD;
use penguin_utils::once::Once;
use penguin_utils::{alignment::align_up, lazy::Lazy};

cpu_local! {
    static ref CURRENT: Lazy<Arc<Process>> = Lazy::new();
}

cpu_local! {
    static ref IDLE_THREAD: Lazy<Arc<Process>> = Lazy::new();
}

cpu_local! {
    static ref HELD_LOCKS: ArrayVec<[Arc<Process>; 2]> = ArrayVec::new();
}

static SCHEDULER: Once<SpinLock<Scheduler>> = Once::new();
static NEXT_PID: AtomicI32 = AtomicI32::new(1);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PId(i32);

impl PId {
    pub const fn new(pid: i32) -> PId {
        PId(pid)
    }
}

fn alloc_pid() -> Option<PId> {
    Some(PId::new(NEXT_PID.fetch_add(1, Ordering::SeqCst)))
}

fn push_bytes_to_stack(sp: &mut VAddr, stack_bottom: VAddr, buf: &[u8]) -> Result<()> {
    if sp.sub(buf.len()) < stack_bottom {
        return Err(Error::with_message(Errno::E2BIG, "too big argvp/envp/auxv"));
    }

    *sp = sp.sub(buf.len());
    sp.write_bytes(buf);
    Ok(())
}

fn push_usize_to_stack(sp: &mut VAddr, stack_bottom: VAddr, value: usize) -> Result<()> {
    if cfg!(target_endian = "big") {
        push_bytes_to_stack(sp, stack_bottom, &value.to_be_bytes())?;
    } else {
        push_bytes_to_stack(sp, stack_bottom, &value.to_le_bytes())?;
    }

    Ok(())
}

#[repr(usize)]
pub enum Auxv {
    Null,
}

fn push_auxv_entry_to_stack(sp: &mut VAddr, stack_bottom: VAddr, auxv: &Auxv) -> Result<()> {
    let (auxv_type, value) = match auxv {
        Auxv::Null => (0, 0),
    };

    push_usize_to_stack(sp, stack_bottom, value)?;
    push_usize_to_stack(sp, stack_bottom, auxv_type)?;
    Ok(())
}

fn estimate_user_init_stack_size(argv: &[&[u8]], envp: &[&[u8]], auxv: &[Auxv]) -> usize {
    let str_len = align_up(
        argv.iter().fold(0, |l, arg| l + arg.len() + 1)
            + envp.iter().fold(0, |l, env| l + env.len() + 1),
        size_of::<usize>(),
    );

    let aux_data_len = auxv.iter().fold(0, |l, aux| {
        l + match aux {
            Auxv::Null => 0,
        }
    });

    let ptrs_len =
        (2 * (1 + auxv.len()) + argv.len() + 1 + envp.len() + 1 + 1) * size_of::<usize>();

    str_len + aux_data_len + ptrs_len
}

/// Initializes a user stack. See "Initial Process Stack" in <https://uclibc.org/docs/psABI-x86_64.pdf>.
fn init_user_stack(
    user_stack_top: UserVAddr,
    stack_top: VAddr,
    stack_bottom: VAddr,
    argv: &[&[u8]],
    envp: &[&[u8]],
    auxv: &[Auxv],
) -> Result<UserVAddr> {
    let mut sp = stack_top;
    let kernel_sp_to_user_sp = |sp: VAddr| {
        let offset = stack_top.value() - sp.value();
        user_stack_top.sub(offset)
    };

    // Write envp strings.
    let mut envp_ptrs = Vec::with_capacity(argv.len());
    for env in envp {
        push_bytes_to_stack(&mut sp, stack_bottom, &[0]);
        push_bytes_to_stack(&mut sp, stack_bottom, env);
        envp_ptrs.push(kernel_sp_to_user_sp(sp)?);
    }

    // Write argv strings.
    let mut argv_ptrs = Vec::with_capacity(argv.len());
    for arg in argv {
        push_bytes_to_stack(&mut sp, stack_bottom, &[0]);
        push_bytes_to_stack(&mut sp, stack_bottom, arg);
        argv_ptrs.push(kernel_sp_to_user_sp(sp)?);
    }

    // The length of the string table wrote above could be unaligned.
    sp.align_down(size_of::<usize>());

    // Push auxiliary vector entries.
    push_auxv_entry_to_stack(&mut sp, stack_bottom, &Auxv::Null);
    for aux in auxv {
        push_auxv_entry_to_stack(&mut sp, stack_bottom, aux);
    }

    // Push environment pointers (`const char **envp`).
    push_usize_to_stack(&mut sp, stack_bottom, 0);
    for ptr in envp_ptrs {
        push_usize_to_stack(&mut sp, stack_bottom, ptr.value());
    }

    // Push argument pointers (`const char **argv`).
    push_usize_to_stack(&mut sp, stack_bottom, 0);
    for ptr in argv_ptrs {
        push_usize_to_stack(&mut sp, stack_bottom, ptr.value());
    }

    // Push argc.
    push_usize_to_stack(&mut sp, stack_bottom, argv.len());

    Ok(kernel_sp_to_user_sp(sp)?)
}

struct ProcessInner {
    arch: arch::Thread,
}

pub struct Process {
    pub pid: PId,
    pub vm: Option<Arc<SpinLock<Vm>>>,
    inner: SpinLock<ProcessInner>,
}

impl Process {
    pub fn new_kthread(ip: VAddr) -> Result<Arc<Process>> {
        let stack_bottom = alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE)
            .into_error_with_message(Errno::ENOMEM, "failed to allocate kernel stack")?;
        let sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);
        let process = Arc::new(Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_kthread(ip, sp),
            }),
            vm: None,
            pid: alloc_pid().into_error_with_message(Errno::EAGAIN, "failed to allocate PID")?,
        });

        SCHEDULER.lock().enqueue(process.clone());
        Ok(process)
    }

    pub fn new_idle_thread() -> Result<Arc<Process>> {
        Ok(Arc::new(Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_idle_thread(),
            }),
            vm: None,
            pid: PId::new(0),
        }))
    }

    pub fn new_init_process(executable: Arc<dyn FileLike>) -> Result<Arc<Process>> {
        // Read the ELF header in the executable file.
        let mut buf = vec![0; 1024];
        executable.read(0, &mut buf)?;

        let elf = Elf::parse(&buf);
        let ip = elf.entry()?;
        let sp = UserVAddr::new(0xdead_0000_beef_beef)?;

        let mut end_of_image = 0;
        for phdr in elf.program_headers() {
            if phdr.p_type == PT_LOAD {
                end_of_image = max(end_of_image, (phdr.p_vaddr + phdr.p_memsz) as usize);
            }
        }

        // Set up the user stack.
        let argv = &[];
        let envp = &[];
        let auxv = &[];
        const USER_STACK_LEN: usize = 128 * 1024; // TODO: Implement rlimit
        let user_stack_bottom = USER_STACK_TOP.sub(USER_STACK_LEN).unwrap().value();
        let user_heap_bottom = align_up(end_of_image, PAGE_SIZE);
        let init_stack_len = align_up(estimate_user_init_stack_size(argv, envp, auxv), PAGE_SIZE);
        if user_heap_bottom >= user_stack_bottom || init_stack_len >= USER_STACK_LEN {
            return Err(Error::new(Errno::E2BIG));
        }

        let init_stack_pages = alloc_pages(init_stack_len / PAGE_SIZE).into_error(Errno::ENOMEM)?;
        let user_sp = init_user_stack(
            USER_STACK_TOP,
            init_stack_pages.as_vaddr().add(init_stack_len),
            init_stack_pages.as_vaddr(),
            argv,
            envp,
            auxv,
        )?;

        let mut vm = Vm::new(
            UserVAddr::new(user_stack_bottom).unwrap(),
            UserVAddr::new(user_heap_bottom).unwrap(),
        );
        for i in 0..(init_stack_len / PAGE_SIZE) {
            vm.page_table_mut().map_user_page(
                USER_STACK_TOP
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
                UserVAddr::new(phdr.p_vaddr as usize)?,
                phdr.p_memsz as usize,
                area_type,
            );
        }

        let stack_bottom = alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE)
            .into_error_with_message(Errno::ENOMEM, "failed to allocate kernel stack")?;

        let kernel_sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);

        let process = Arc::new(Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_user_thread(ip, user_sp, kernel_sp),
            }),
            vm: Some(Arc::new(SpinLock::new(vm))),
            pid: PId::new(1),
        });

        SCHEDULER.lock().enqueue(process.clone());
        Ok(process)
    }
}

pub struct Scheduler {
    run_queue: VecDeque<Arc<Process>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            run_queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, thread: Arc<Process>) {
        self.run_queue.push_back(thread);
    }

    pub fn pick_next(&mut self) -> Option<Arc<Process>> {
        self.run_queue.pop_front()
    }
}

pub fn current_process() -> &'static Arc<Process> {
    CURRENT.get()
}

/// Yields execution to another thread. When the currently running thread is resumed
// in future, it will be
pub fn switch() {
    // Save the current interrupt enable flag to restore it in the next execution
    // of the currently running thread.
    let interrupt_enabled = is_interrupt_enabled();

    let prev_thread = CURRENT.get();
    let next_thread = {
        let mut scheduler = SCHEDULER.lock();

        // Push back the currently running thread to the runqueue if it's still
        // ready for running, in other words, it's not blocked.
        if prev_thread.pid != PId::new(0) {
            scheduler.enqueue((*prev_thread).clone());
        }

        // Pick a thread to run next.
        match scheduler.pick_next() {
            Some(next) => next,
            None => IDLE_THREAD.get().get().clone(),
        }
    };

    assert!(HELD_LOCKS.get().is_empty());
    assert!(!Arc::ptr_eq(prev_thread, &next_thread));

    // Save locks that will be released later.
    debug_assert!(HELD_LOCKS.get().is_empty());
    HELD_LOCKS.as_mut().push((*prev_thread).clone());
    HELD_LOCKS.as_mut().push(next_thread.clone());

    // Since these locks won't be dropped until the current (prev) thread is
    // resumed next time, we'll unlock these locks in `after_switch` in the next
    // thread's context.
    let mut prev_inner = prev_thread.inner.lock();
    let mut next_inner = next_thread.inner.lock();

    if let Some(vm) = next_thread.vm.as_ref() {
        let lock = vm.lock();
        lock.page_table().switch();
    }

    // Switch into the next thread.
    CURRENT.as_mut().set(next_thread.clone());
    arch::switch_thread(&mut (*prev_inner).arch, &mut (*next_inner).arch);

    // Don't call destructors as they're unlocked in `after_switch`.
    mem::forget(prev_inner);
    mem::forget(next_inner);

    // Now we're in the next thread. Release held locks and continue executing.
    after_switch();

    // Retstore the interrupt enable flag manually because lock guards
    // (`prev` and `next`) that holds the flag state are `mem::forget`-ed.
    if interrupt_enabled {
        unsafe {
            enable_interrupt();
        }
    }
}

#[no_mangle]
pub extern "C" fn after_switch() {
    for thread in HELD_LOCKS.as_mut().drain(..) {
        unsafe {
            thread.inner.force_unlock();
        }
    }
}

static mut COUNT: usize = 0;

fn thread_a() {
    loop {
        if unsafe { COUNT } % 50 == 0 {
            arch::printchar('\n');
        }
        unsafe { COUNT += 1 };
        arch::printchar('A');
        for _ in 0..0x10000 {
            unsafe {
                asm!("nop; pause");
            }
        }
    }
}

fn thread_b() {
    loop {
        if unsafe { COUNT } % 50 == 0 {
            arch::printchar('\n');
        }
        unsafe { COUNT += 1 };
        arch::printchar('B');
        for _ in 0..0x10000 {
            unsafe {
                asm!("nop; pause");
            }
        }
    }
}

fn thread_c() {
    loop {
        if unsafe { COUNT } % 50 == 0 {
            arch::printchar('\n');
        }
        unsafe { COUNT += 1 };
        arch::printchar('C');
        for _ in 0..0x10000 {
            unsafe {
                asm!("nop; pause");
            }
        }
    }
}

struct DummyFile(&'static [u8]);
impl FileLike for DummyFile {
    fn read(&self, offset: usize, buf: &mut [u8]) -> crate::result::Result<usize> {
        let end = core::cmp::min(offset + buf.len(), self.0.len());
        let copy_len = end - offset;
        buf[..copy_len].copy_from_slice(&self.0[offset..end]);
        Ok(copy_len)
    }

    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        todo!()
    }

    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }
}

pub fn init() {
    SCHEDULER.init(|| SpinLock::new(Scheduler::new()));
    let idle_thread = Process::new_idle_thread().unwrap();
    IDLE_THREAD.as_mut().set(idle_thread.clone());
    CURRENT.as_mut().set(idle_thread);

    let root_fs = RootFs::new(INITRAM_FS.clone());
    let root_dir = root_fs.root_dir().expect("failed to open the root dir");
    let inode = root_fs
        .lookup_inode(root_dir, Path::new("/sbin/init"))
        .expect("failed to open /sbin/init");
    let file = match inode {
        INode::FileLike(file) => file,
        _ => panic!("/sbin/init is not a file"),
    };
    Process::new_init_process(file).unwrap();

    Process::new_kthread(VAddr::new(thread_a as *const u8 as usize)).unwrap();
    Process::new_kthread(VAddr::new(thread_b as *const u8 as usize)).unwrap();
    Process::new_kthread(VAddr::new(thread_c as *const u8 as usize)).unwrap();
}

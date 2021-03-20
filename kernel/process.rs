use crate::{
    arch::{self, disable_interrupt, enable_interrupt, is_interrupt_enabled, SpinLock, VAddr},
    elf::Elf,
    fs::inode::FileLike,
    mm::{
        page_allocator::alloc_pages,
        vm::{Vm, VmAreaType},
    },
};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use arch::{UserVAddr, KERNEL_STACK_SIZE, PAGE_SIZE};
use arrayvec::ArrayVec;
use core::mem;
use core::sync::atomic::{AtomicI32, Ordering};
use penguin_utils::lazy::Lazy;
use penguin_utils::once::Once;

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

struct ProcessInner {
    arch: arch::Thread,
}

pub struct Process {
    pub pid: PId,
    pub vm: Option<Arc<SpinLock<Vm>>>,
    inner: SpinLock<ProcessInner>,
}

impl Process {
    pub fn new_kthread(ip: VAddr) -> Process {
        // FIXME: Return an error instead of panic'ing.
        let stack_bottom =
            alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE).expect("failed to allocate kernel stack");
        let sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);
        Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_kthread(ip, sp),
            }),
            vm: None,
            pid: alloc_pid().expect("failed to allocate PID"),
        }
    }

    pub fn new_idle_thread() -> Process {
        Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_idle_thread(),
            }),
            vm: None,
            pid: PId::new(0),
        }
    }

    pub fn new_init_process(executable: Arc<dyn FileLike>) -> Process {
        // FIXME: Return an error instead of panic'ing.

        // Read the ELF header in the executable file.
        let mut buf = vec![0; 1024];
        executable
            .read(0, &mut buf)
            .expect("failed to read executable");

        let elf = Elf::parse(&buf);
        let ip = elf.entry();
        let sp = UserVAddr::new(0xdead_0000_beef_beef);

        // Register program headers in the virtual memory space.
        let mut vm = Vm::new();
        for phdr in elf.program_headers() {
            // TODO: Ignore non-alloc headers.
            assert!(phdr.p_memsz == phdr.p_filesz); // FIXME:
            vm.add_vm_area(
                UserVAddr::new(phdr.p_vaddr as usize),
                phdr.p_filesz as usize,
                VmAreaType::File {
                    file: executable.clone(),
                    offset: phdr.p_offset as usize,
                },
            );
        }

        let stack_bottom =
            alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE).expect("failed to allocate kernel stack");
        let kernel_sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);

        Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_user_thread(ip, sp, kernel_sp),
            }),
            vm: Some(Arc::new(SpinLock::new(vm))),
            pid: PId::new(1),
        }
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

/*
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
*/

struct DummyFile(&'static [u8]);
impl FileLike for DummyFile {
    fn read(&self, offset: usize, buf: &mut [u8]) -> crate::result::Result<usize> {
        buf.copy_from_slice(&self.0[offset..offset + buf.len()]);
        Ok(buf.len())
    }
}

pub fn init() {
    SCHEDULER.init(|| SpinLock::new(Scheduler::new()));
    let idle_thread = Arc::new(Process::new_idle_thread());
    IDLE_THREAD.as_mut().set(idle_thread.clone());
    CURRENT.as_mut().set(idle_thread);

    let file = DummyFile(include_bytes!("../hello_world.elf"));

    let init_process = Process::new_init_process(Arc::new(file));

    SCHEDULER.lock().enqueue(Arc::new(init_process));

    /*
    let mut thread_a = Process::new_kthread(VAddr::new(thread_a as *const u8 as usize));
    let mut thread_b = Process::new_kthread(VAddr::new(thread_b as *const u8 as usize));
    let mut thread_c = Process::new_kthread(VAddr::new(thread_c as *const u8 as usize));
    SCHEDULER.lock().enqueue(Arc::new((thread_a)));
    SCHEDULER.lock().enqueue(Arc::new((thread_b)));
    SCHEDULER.lock().enqueue(Arc::new((thread_c)));
    */
}

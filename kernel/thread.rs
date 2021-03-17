use crate::arch::{
    self, disable_interrupt, enable_interrupt, is_interrupt_enabled, SpinLock, VAddr,
};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use arrayvec::ArrayVec;
use core::mem;
use core::sync::atomic::{AtomicI32, Ordering};
use penguin_utils::lazy::Lazy;
use penguin_utils::once::Once;

cpu_local! {
    static ref CURRENT_THREAD: Lazy<Arc<SpinLock<Thread>>> = Lazy::new();
}

cpu_local! {
    static ref IDLE_THREAD: Lazy<Arc<SpinLock<Thread>>> = Lazy::new();
}

cpu_local! {
    static ref HELD_LOCKS: ArrayVec<[Arc<SpinLock<Thread>>; 2]> = ArrayVec::new();
}

static SCHEDULER: Once<SpinLock<Scheduler>> = Once::new();
static THREADS: SpinLock<BTreeMap<PId, Thread>> = SpinLock::new(BTreeMap::new());
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

pub struct Thread {
    pub arch: arch::Thread,
    pub pid: PId,
}

impl Thread {
    pub fn new_kthread(ip: VAddr, sp: VAddr) -> Thread {
        Thread {
            arch: arch::Thread::new_kthread(ip, sp),
            pid: alloc_pid().expect("failed to allocate PID"),
        }
    }

    pub fn new_idle_thread() -> Thread {
        Thread {
            arch: arch::Thread::new_idle_thread(),
            pid: PId::new(0),
        }
    }
}

pub struct Scheduler {
    run_queue: VecDeque<Arc<SpinLock<Thread>>>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            run_queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, thread: Arc<SpinLock<Thread>>) {
        self.run_queue.push_back(thread);
    }

    pub fn pick_next(&mut self) -> Option<Arc<SpinLock<Thread>>> {
        self.run_queue.pop_front()
    }
}

/// Yields execution to another thread. When the currently running thread is resumed
// in future, it will be
pub fn switch_thread() {
    // Save the current interrupt enable flag to restore it in the next execution
    // of the currently running thread.
    let interrupt_enabled = is_interrupt_enabled();

    let prev_lock = CURRENT_THREAD.get();
    let next_lock = {
        let mut scheduler = SCHEDULER.lock();

        // Push back the currently running thread to the runqueue if it's still
        // ready for running, in other words, it's not blocked.
        if prev_lock.lock().pid != PId::new(0) {
            scheduler.enqueue((*prev_lock).clone());
        }

        // Pick a thread to run next.
        match scheduler.pick_next() {
            Some(next) => next,
            None => IDLE_THREAD.get().get().clone(),
        }
    };

    assert!(HELD_LOCKS.get().is_empty());
    assert!(!Arc::ptr_eq(prev_lock, &next_lock));

    // Save locks that will be released later.
    debug_assert!(HELD_LOCKS.get().is_empty());
    HELD_LOCKS.as_mut().push((*prev_lock).clone());
    HELD_LOCKS.as_mut().push(next_lock.clone());

    // Since these locks won't be dropped until the current (prev) thread is
    // resumed next time, we'll unlock these locks in `after_switch` in the next
    // thread's context.
    let mut prev = prev_lock.lock();
    let mut next = next_lock.lock();

    // Switch into the next thread.
    CURRENT_THREAD.as_mut().set(next_lock.clone());
    arch::switch_thread(&mut (*prev).arch, &mut (*next).arch);

    // Don't call destructors as they're unlocked in `after_switch`.
    mem::forget(prev);
    mem::forget(next);

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
    for lock in HELD_LOCKS.as_mut().drain(..) {
        unsafe {
            lock.force_unlock();
        }
    }
}

static stack_a: [u8; 16 * 1024] = [0; 16 * 1024];
static stack_b: [u8; 16 * 1024] = [0; 16 * 1024];
static stack_c: [u8; 16 * 1024] = [0; 16 * 1024];
static mut count: usize = 0;

fn thread_a() {
    loop {
        if unsafe { count } % 50 == 0 {
            arch::printchar('\n');
        }
        unsafe { count += 1 };
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
        if unsafe { count } % 50 == 0 {
            arch::printchar('\n');
        }
        unsafe { count += 1 };
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
        if unsafe { count } % 50 == 0 {
            arch::printchar('\n');
        }
        unsafe { count += 1 };
        arch::printchar('C');
        for _ in 0..0x10000 {
            unsafe {
                asm!("nop; pause");
            }
        }
    }
}

pub fn init() {
    SCHEDULER.init(|| SpinLock::new(Scheduler::new()));
    let idle_thread = Arc::new(SpinLock::new(Thread::new_idle_thread()));
    IDLE_THREAD.as_mut().set(idle_thread.clone());
    CURRENT_THREAD.as_mut().set(idle_thread);

    let mut thread_a = Thread::new_kthread(
        VAddr::new(thread_a as *const u8 as usize),
        VAddr::new(((&stack_a as *const u8 as usize) + stack_a.len()) as usize),
    );
    let mut thread_b = Thread::new_kthread(
        VAddr::new(thread_b as *const u8 as usize),
        VAddr::new(((&stack_b as *const u8 as usize) + stack_b.len()) as usize),
    );
    let mut thread_c = Thread::new_kthread(
        VAddr::new(thread_c as *const u8 as usize),
        VAddr::new(((&stack_c as *const u8 as usize) + stack_c.len()) as usize),
    );

    SCHEDULER.lock().enqueue(Arc::new(SpinLock::new(thread_a)));
    SCHEDULER.lock().enqueue(Arc::new(SpinLock::new(thread_b)));
    SCHEDULER.lock().enqueue(Arc::new(SpinLock::new(thread_c)));
}

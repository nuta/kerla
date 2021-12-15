use alloc::sync::Arc;

use kerla_runtime::{address::UserVAddr, spinlock::SpinLock};

use kerla_utils::lazy::Lazy;
use kerla_utils::once::Once;

mod cmdline;
mod elf;
mod init_stack;
#[allow(clippy::module_inception)]
mod process;
pub mod process_group;
mod scheduler;
pub mod signal;
mod switch;
mod wait_queue;

pub use process::{gc_exited_processes, read_process_stats, PId, Process, ProcessState};
pub use switch::switch;
pub use wait_queue::WaitQueue;

use self::scheduler::Scheduler;

cpu_local! {
    static ref CURRENT: Lazy<Arc<Process>> = Lazy::new();
}

cpu_local! {
    // TODO: Should be pub(super)
    pub static ref IDLE_THREAD: Lazy<Arc<Process>> = Lazy::new();
}

static SCHEDULER: Once<SpinLock<Scheduler>> = Once::new();
pub static JOIN_WAIT_QUEUE: Once<WaitQueue> = Once::new();

pub fn current_process() -> &'static Arc<Process> {
    CURRENT.get()
}

pub fn init() {
    JOIN_WAIT_QUEUE.init(WaitQueue::new);
    SCHEDULER.init(|| SpinLock::new(Scheduler::new()));
    let idle_thread = Process::new_idle_thread().unwrap();
    IDLE_THREAD.as_mut().set(idle_thread.clone());
    CURRENT.as_mut().set(idle_thread);
}

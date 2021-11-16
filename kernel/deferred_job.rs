//! A deferred job queue.
//!
//! When you want to run some time-consuming work, please consider using this
//! mechanism.
use alloc::boxed::Box;
use crossbeam::queue::SegQueue;
use kerla_api::sync::SpinLock;

pub trait JobCallback = FnOnce() + Send + 'static;
static GLOBAL_QUEUE: SpinLock<SegQueue<Box<dyn JobCallback>>> = SpinLock::new(SegQueue::new());

pub struct DeferredJob {
    // Will be useful for debugging.
    #[allow(unused)]
    name: &'static str,
}

impl DeferredJob {
    pub const fn new(name: &'static str) -> DeferredJob {
        DeferredJob { name }
    }

    /// Enqueues a job. `callback` will be automatically run sometime later.
    ///
    /// # Caveats
    ///
    /// `callback` MUST NOT sleep since it's can be run in an interrupt context!
    pub fn run_later<F: JobCallback>(&self, callback: F) {
        GLOBAL_QUEUE.lock().push(Box::new(callback));
    }
}

/// Run pending deferred jobs.
pub fn run_deferred_jobs() {
    // TODO: The current user process is still blocked until we leave the
    //       interrupt handler. Should we have a limit of the maximum number of jobs?
    //
    // TODO: Re-enable interrupts here since this may take long.
    while let Some(callback) = GLOBAL_QUEUE.lock().pop() {
        callback();
    }
}

use crate::{
    arch::{enable_irq, SpinLock},
    net::process_packets,
    process::{self, ProcessState},
};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};

const PREEMPT_MS: usize = 30;
static TICKS: AtomicUsize = AtomicUsize::new(0);

pub fn handle_timer_irq() {
    let ticks = TICKS.fetch_add(1, Ordering::Relaxed);
    if ticks % PREEMPT_MS == 0 {
        process::switch(ProcessState::Runnable);
    }
}

// TODO: Use a simple array for faster access.
static IRQ_HANDLERS: SpinLock<BTreeMap<u8, Box<dyn FnMut() + Send + Sync>>> =
    SpinLock::new(BTreeMap::new());

pub fn attach_irq<F: FnMut() + Send + Sync + 'static>(irq: u8, f: F) {
    IRQ_HANDLERS.lock().insert(irq, Box::new(f));
    enable_irq(irq);
}

pub fn handle_irq(irq: u8) {
    if let Some(handler) = IRQ_HANDLERS.lock().get_mut(&irq) {
        (*handler)();
        process_packets();
    }
}

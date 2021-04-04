use crate::process::{self, ProcessState};
use core::sync::atomic::{AtomicUsize, Ordering};

const PREEMPT_MS: usize = 30;
static TICKS: AtomicUsize = AtomicUsize::new(0);

pub fn handle_timer_irq() {
    let ticks = TICKS.fetch_add(1, Ordering::Relaxed);
    if ticks % PREEMPT_MS == 0 {
        process::switch(ProcessState::Runnable);
    }
}

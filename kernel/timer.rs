use crate::{
    arch::{SpinLock, TICK_HZ},
    process::{self, Process, ProcessState},
};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use process::{current_process, resume, switch};

const PREEMPT_PER_TICKS: usize = 30;
static MONOTONIC_TICKS: AtomicUsize = AtomicUsize::new(0);
/// Ticks from the epoch (00:00:00 on 1 January 1970, UTC).
static WALLCLOCK_TICKS: AtomicUsize = AtomicUsize::new(0);
static TIMERS: SpinLock<Vec<Timer>> = SpinLock::new(Vec::new());

struct Timer {
    current: usize,
    reset: Option<usize>,
    process: Arc<Process>,
}

/// Suspends the current process at least `ms` milliseconds.
pub fn sleep_ms(ms: usize) {
    TIMERS.lock().push(Timer {
        current: ms * TICK_HZ / 1000,
        reset: None,
        process: current_process().clone(),
    });

    switch(ProcessState::Sleeping);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct WallClock {
    ticks_from_epoch: usize,
}

impl WallClock {
    pub fn secs_from_epoch(self) -> usize {
        self.ticks_from_epoch / TICK_HZ
    }

    pub fn msecs_from_epoch(self) -> usize {
        self.ticks_from_epoch / (TICK_HZ / 1000)
    }

    pub fn nanosecs_from_epoch(self) -> usize {
        self.msecs_from_epoch() * 1_000_000
    }
}

pub fn read_wall_clock() -> WallClock {
    WallClock {
        ticks_from_epoch: WALLCLOCK_TICKS.load(Ordering::Relaxed),
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MonotonicClock {
    ticks: usize,
}

impl MonotonicClock {
    pub fn secs(self) -> usize {
        self.ticks / TICK_HZ
    }

    pub fn msecs(self) -> usize {
        self.ticks / (TICK_HZ / 1000)
    }
}

pub fn read_monotonic_clock() -> MonotonicClock {
    MonotonicClock {
        ticks: MONOTONIC_TICKS.load(Ordering::Relaxed),
    }
}

pub fn handle_timer_irq() {
    {
        let mut timers = TIMERS.lock();
        for timer in timers.iter_mut() {
            timer.current -= 1;
        }

        timers.retain(|timer| {
            if timer.current == 0 {
                resume(&timer.process);
            }

            timer.current > 0
        })
    }

    WALLCLOCK_TICKS.fetch_add(1, Ordering::Relaxed);
    let ticks = MONOTONIC_TICKS.fetch_add(1, Ordering::Relaxed);
    if ticks % PREEMPT_PER_TICKS == 0 {
        process::switch(ProcessState::Runnable);
    }
}

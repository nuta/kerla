use crate::{
    arch::TICK_HZ,
    process::{self, ProcessState},
};
use core::sync::atomic::{AtomicUsize, Ordering};

const PREEMPT_PER_TICKS: usize = 30;
static MONOTONIC_TICKS: AtomicUsize = AtomicUsize::new(0);
/// Ticks from the epoch (00:00:00 on 1 January 1970, UTC).
static WALLCLOCK_TICKS: AtomicUsize = AtomicUsize::new(0);

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
    WALLCLOCK_TICKS.fetch_add(1, Ordering::Relaxed);
    let ticks = MONOTONIC_TICKS.fetch_add(1, Ordering::Relaxed);
    if ticks % PREEMPT_PER_TICKS == 0 {
        process::switch(ProcessState::Runnable);
    }
}

use crate::{
    ctypes::*,
    prelude::*,
    process::{self, current_process, Process, ProcessState},
};
use core::sync::atomic::{AtomicUsize, Ordering};
use kerla_runtime::{arch::TICK_HZ, spinlock::SpinLock};
use process::switch;

const PREEMPT_PER_TICKS: usize = 30;
static MONOTONIC_TICKS: AtomicUsize = AtomicUsize::new(0);
/// Ticks from the epoch (00:00:00 on 1 January 1970, UTC).
static WALLCLOCK_TICKS: AtomicUsize = AtomicUsize::new(0);
static TIMERS: SpinLock<Vec<Timer>> = SpinLock::new(Vec::new());

struct Timer {
    current: usize,
    process: Arc<Process>,
}

/// Suspends the current process at least `ms` milliseconds.
pub fn _sleep_ms(ms: usize) {
    TIMERS.lock().push(Timer {
        current: ms * TICK_HZ / 1000,
        process: current_process().clone(),
    });

    current_process().set_state(ProcessState::BlockedSignalable);
    switch();
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

    pub fn nanosecs(self) -> usize {
        self.msecs() * 1_000_000
    }

    pub fn elapsed_msecs(self) -> usize {
        // FIXME: Consider wrapping.
        (read_monotonic_clock().ticks - self.ticks) / (TICK_HZ / 1000)
    }
}

pub fn read_monotonic_clock() -> MonotonicClock {
    MonotonicClock {
        ticks: MONOTONIC_TICKS.load(Ordering::Relaxed),
    }
}

/// `struct timeval`
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct Timeval {
    tv_sec: c_time,
    tv_usec: c_suseconds,
}

impl Timeval {
    pub fn as_msecs(&self) -> usize {
        (self.tv_sec as usize) * 1000 + (self.tv_usec as usize) / 1000
    }
}

pub fn handle_timer_irq() {
    {
        let mut timers = TIMERS.lock();
        for timer in timers.iter_mut() {
            if timer.current > 0 {
                timer.current -= 1;
            }
        }

        timers.retain(|timer| {
            if timer.current == 0 {
                timer.process.resume();
            }

            timer.current > 0
        })
    }

    WALLCLOCK_TICKS.fetch_add(1, Ordering::Relaxed);
    let ticks = MONOTONIC_TICKS.fetch_add(1, Ordering::Relaxed);
    if ticks % PREEMPT_PER_TICKS == 0 {
        process::switch();
    }
}

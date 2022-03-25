//! An OS-agnostic bootstrap and runtime support library for operating system
//! kernels.
#![no_std]

extern crate alloc;

#[macro_use]
extern crate log;

#[macro_use]
pub mod print;

pub mod address;
pub mod backtrace;
pub mod bootinfo;
pub mod global_allocator;
pub mod logger;
pub mod page_allocator;
pub mod profile;
pub mod spinlock;

mod x64;

pub mod arch {
    #[cfg(target_arch = "x86_64")]
    pub use super::x64::{
        enable_irq, halt, idle, read_clock_counter, semihosting_halt, x64_specific, Backtrace,
        PageFaultReason, PageTable, PtRegs, SavedInterruptStatus, SemihostingExitStatus,
        KERNEL_BASE_ADDR, KERNEL_STRAIGHT_MAP_PADDR_END, PAGE_SIZE, TICK_HZ,
    };
}

use address::UserVAddr;
use kerla_utils::static_cell::StaticCell;

pub trait Handler: Sync {
    fn handle_console_rx(&self, char: u8);
    fn handle_irq(&self, irq: u8);
    fn handle_timer_irq(&self);
    fn handle_page_fault(
        &self,
        unaligned_vaddr: Option<UserVAddr>,
        ip: usize,
        _reason: arch::PageFaultReason,
    );

    #[allow(clippy::too_many_arguments)]
    fn handle_syscall(
        &self,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        a5: usize,
        a6: usize,
        n: usize,
        frame: *mut arch::PtRegs,
    ) -> isize;

    #[cfg(debug_assertions)]
    fn usercopy_hook(&self) {}
}

static HANDLER: StaticCell<&dyn Handler> = StaticCell::new(&NopHandler);

struct NopHandler;

impl Handler for NopHandler {
    fn handle_console_rx(&self, _char: u8) {}
    fn handle_irq(&self, _irq: u8) {}
    fn handle_timer_irq(&self) {}

    fn handle_page_fault(
        &self,
        _unaligned_vaddr: Option<UserVAddr>,
        _ip: usize,
        _reason: arch::PageFaultReason,
    ) {
    }

    fn handle_syscall(
        &self,
        _a1: usize,
        _a2: usize,
        _a3: usize,
        _a4: usize,
        _a5: usize,
        _a6: usize,
        _n: usize,
        _frame: *mut arch::PtRegs,
    ) -> isize {
        0
    }
}

fn handler() -> &'static dyn Handler {
    HANDLER.load()
}

pub fn set_handler(handler: &'static dyn Handler) {
    HANDLER.store(handler);
}

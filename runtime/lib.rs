#![no_std]
#![feature(asm)]
#![feature(global_asm)]

extern crate alloc;

#[macro_use]
extern crate log;

#[macro_use]
pub mod printk;

pub mod address;
pub mod backtrace;
pub mod bootinfo;
pub mod global_allocator;
pub mod logger;
pub mod page_allocator;
pub mod spinlock;

pub use address::{PAddr, UserVAddr, VAddr};
pub use printk::print_bytes;
pub use spinlock::{SpinLock, SpinLockGuard};

pub mod x64;
pub use x64::*;

use kerla_utils::static_cell::StaticCell;

pub trait Handler: Sync {
    fn handle_console_rx(&self, char: u8);
    fn handle_irq(&self, irq: u8);
    fn handle_timer_irq(&self);
    fn handle_page_fault(
        &self,
        unaligned_vaddr: Option<UserVAddr>,
        ip: usize,
        _reason: PageFaultReason,
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
        frame: *mut SyscallFrame,
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
        _reason: PageFaultReason,
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
        _frame: *mut x64::SyscallFrame,
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

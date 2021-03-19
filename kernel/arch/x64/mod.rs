global_asm!(include_str!("boot.S"));
global_asm!(include_str!("trap.S"));

#[macro_use]
mod cpu_local;
mod address;
mod apic;
mod backtrace;
mod boot;
mod gdt;
mod idle;
mod idt;
mod interrupt;
mod ioapic;
mod lock;
mod multiboot;
mod page_table;
mod pit;
mod semihosting;
mod serial;
mod syscall;
mod thread;
mod tss;

pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 16;
pub const PAGE_SIZE: usize = 4096;

pub use address::{PAddr, UserVAddr, VAddr};
pub use backtrace::Backtrace;
pub use idle::{halt, idle};
pub use interrupt::{disable_interrupt, enable_interrupt, is_interrupt_enabled};
pub use lock::{SpinLock, SpinLockGuard};
#[cfg(test)]
pub use semihosting::{semihosting_halt, ExitStatus};
pub use serial::printchar;
pub use thread::{switch_thread, Thread};

global_asm!(include_str!("boot.S"));
global_asm!(include_str!("trap.S"));
global_asm!(include_str!("usercopy.S"));

#[macro_use]
mod cpu_local;
mod address;
mod apic;
mod arch_prctl;
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

pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 256;
pub const USER_STACK_TOP: UserVAddr = unsafe { UserVAddr::new_unchecked(0x0000_0c00_0000_0000) };
pub const PAGE_SIZE: usize = 4096;

pub use address::{PAddr, UserVAddr, VAddr};
pub use arch_prctl::arch_prctl;
pub use backtrace::Backtrace;
pub use boot::init;
pub use idle::{halt, idle};
pub use interrupt::{enable_interrupt, is_interrupt_enabled};
pub use lock::{SpinLock, SpinLockGuard};
pub use page_table::{PageFaultReason, PageTable};
#[cfg(test)]
pub use semihosting::{semihosting_halt, ExitStatus};
pub use serial::{print_str, printchar};
pub use thread::{switch_thread, Thread};

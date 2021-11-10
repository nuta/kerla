#[macro_use]
mod cpu_local;

mod apic;
mod backtrace;
mod boot;
mod bootinfo;
mod gdt;
mod idle;
mod idt;
mod interrupt;
mod ioapic;
mod paging;
mod pit;
mod profile;
mod semihosting;
mod serial;
mod syscall;
mod tss;

pub use backtrace::Backtrace;
pub use idle::{halt, idle};
pub use interrupt::SavedInterruptStatus;
pub use paging::{PageFaultReason, PageTable};
pub use profile::read_clock_counter;
pub use semihosting::{semihosting_halt, ExitStatus};
pub use syscall::SyscallFrame;

// x64-specific objects.
pub use cpu_local::cpu_local_head;
pub use gdt::{USER_CS32, USER_CS64, USER_DS, USER_RPL};
pub use tss::Tss;

pub const PAGE_SIZE: usize = 4096;

/// The base virtual address of straight mapping.
pub const KERNEL_BASE_ADDR: u64 = 0xffff_8000_0000_0000;

/// The end of straight mapping. Any physical address `P` is mapped into the
/// kernel's virtual memory address `KERNEL_BASE_ADDR + P`.
pub const KERNEL_STRAIGHT_MAP_PADDR_END: u64 = 0x1_0000_0000;

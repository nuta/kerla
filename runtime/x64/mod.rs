use core::arch::global_asm;

global_asm!(include_str!("boot.S"));
global_asm!(include_str!("trap.S"));
global_asm!(include_str!("usercopy.S"));

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
mod vga;

pub use backtrace::Backtrace;
pub use idle::{halt, idle};
pub use interrupt::SavedInterruptStatus;
pub use ioapic::enable_irq;
pub use paging::{PageFaultReason, PageTable};
pub use profile::read_clock_counter;
pub use semihosting::{semihosting_halt, SemihostingExitStatus};
pub use syscall::PtRegs;

pub mod x64_specific {
    pub use super::cpu_local::cpu_local_head;
    pub use super::gdt::{USER_CS32, USER_CS64, USER_DS, USER_RPL};
    pub use super::tss::TSS;
}

pub const PAGE_SIZE: usize = 4096;
pub const TICK_HZ: usize = 1000;

/// The base virtual address of straight mapping.
pub const KERNEL_BASE_ADDR: usize = 0xffff_8000_0000_0000;

/// The end of straight mapping. Any physical address `P` is mapped into the
/// kernel's virtual memory address `KERNEL_BASE_ADDR + P`.
pub const KERNEL_STRAIGHT_MAP_PADDR_END: usize = 0x1_0000_0000;

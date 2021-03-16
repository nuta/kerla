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
mod pit;
mod semihosting;
mod serial;
mod syscall;
mod tss;

pub use address::{PAddr, VAddr};
pub use backtrace::Backtrace;
pub use idle::{halt, idle};
#[cfg(test)]
pub use semihosting::{semihosting_halt, ExitStatus};
pub use serial::printchar;

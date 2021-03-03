global_asm!(include_str!("boot.S"));
global_asm!(include_str!("trap.S"));

mod address;
mod apic;
mod asm;
mod boot;
mod gdt;
mod idle;
mod idt;
mod interrupt;
mod ioapic;
mod lock;
mod multiboot;
mod semihosting;
mod serial;
mod syscall;
mod tss;

pub use idle::{halt, idle};
#[cfg(test)]
pub use semihosting::{semihosting_halt, ExitStatus};
pub use serial::printchar;

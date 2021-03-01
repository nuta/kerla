global_asm!(include_str!("boot.S"));

mod asm;
mod boot;
mod semihosting;
mod serial;

#[cfg(test)]
pub use semihosting::{semihosting_halt, ExitStatus};
pub use serial::printchar;

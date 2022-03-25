use core::arch::asm;

use super::semihosting::{semihosting_halt, SemihostingExitStatus};

pub fn idle() {
    unsafe {
        asm!("sti; hlt; cli");
    }
}

#[cfg_attr(test, allow(unused))]
pub fn halt() -> ! {
    semihosting_halt(SemihostingExitStatus::Success);

    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}

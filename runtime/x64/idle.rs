use super::semihosting::{semihosting_halt, ExitStatus};

pub fn idle() {
    unsafe {
        asm!("sti; hlt");
    }
}

#[cfg_attr(test, allow(unused))]
pub fn halt() -> ! {
    semihosting_halt(ExitStatus::Success);

    loop {
        unsafe {
            asm!("cli; hlt");
        }
    }
}

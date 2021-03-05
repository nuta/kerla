#![cfg(test)]
use x86::io::outw;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Success = 0x10,
    Failure = 0x11,
}

pub fn semihosting_halt(status: ExitStatus) {
    unsafe {
        outw(0x501, status as u16);
    }
}

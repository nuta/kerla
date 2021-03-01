#![cfg(test)]
use super::asm;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Success = 0x10,
    Failure = 0x11,
}

pub fn semihosting_halt(status: ExitStatus) {
    unsafe {
        asm::out16(0x501, status as u16);
    }
}

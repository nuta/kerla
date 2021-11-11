use x86::io::outw;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemihostingExitStatus {
    Success = 0x10,
    Failure = 0x11,
}

pub fn semihosting_halt(status: SemihostingExitStatus) {
    unsafe {
        outw(0x501, status as u16);
    }
}

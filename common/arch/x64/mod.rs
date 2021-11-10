use crate::addr::VAddr;
use x86::current::rflags::{self, RFlags};

pub const PAGE_SIZE: usize = 4096;

pub struct SavedInterruptStatus {
    rflags: RFlags,
}

impl SavedInterruptStatus {
    pub fn save() -> SavedInterruptStatus {
        SavedInterruptStatus {
            rflags: rflags::read(),
        }
    }
}

impl Drop for SavedInterruptStatus {
    fn drop(&mut self) {
        rflags::set(rflags::read() | (self.rflags & rflags::RFlags::FLAGS_IF));
    }
}

const BACKTRACE_MAX: usize = 16;

#[repr(C, packed)]
pub struct StackFrame {
    next: *const StackFrame,
    return_addr: u64,
}

pub struct Backtrace {
    frame: *const StackFrame,
}

impl Backtrace {
    pub fn current_frame() -> Backtrace {
        Backtrace {
            frame: x86::current::registers::rbp() as *const StackFrame,
        }
    }

    pub fn traverse<F>(self, mut callback: F)
    where
        F: FnMut(usize, VAddr),
    {
        let mut frame = self.frame;
        for i in 0..BACKTRACE_MAX {
            if frame.is_null() || !VAddr::is_accessible_from_kernel(frame as usize) {
                break;
            }

            unsafe {
                callback(i, VAddr::new((*frame).return_addr as usize));
                frame = (*frame).next;
            }
        }
    }
}

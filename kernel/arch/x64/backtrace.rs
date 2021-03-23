use super::address::{VAddr, KERNEL_BASE_ADDR};

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

    pub fn traverse<F>(self, callback: F)
    where
        F: Fn(usize, VAddr),
    {
        let mut frame = self.frame;
        for i in 0..BACKTRACE_MAX {
            if frame.is_null() || (frame as u64) < KERNEL_BASE_ADDR {
                break;
            }

            unsafe {
                callback(i, VAddr::new((*frame).return_addr as usize));
                frame = (*frame).next;
            }
        }
    }
}

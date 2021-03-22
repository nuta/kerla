use core::unimplemented;

use super::{
    gdt::{KERNEL_CS, USER_CS32},
    UserVAddr,
};
use x86::msr::{self, rdmsr, wrmsr};

// Clear IF bit to disable interrupts when we enter the syscall handler
// or an interrupt occurs before doing SWAPGS.
const SYSCALL_RFLAGS_MASK: u64 = 0x200;

#[no_mangle]
extern "C" fn x64_handle_syscall(
    a1: i64,
    a2: i64,
    a3: i64,
    a4: i64,
    a5: i64,
    a6: i64,
    n: i64,
) -> i64 {
    println!("syscall: n={}", n);
    match n {
        1 => {
            println!("sys_write({}, {:x}, {})", a1, a2, a3);
            let uaddr = UserVAddr::new(a2 as usize).unwrap();
            let mut buf = vec![0; a3 as usize];
            uaddr.read_bytes(&mut buf);
            println!("write: \x1b[1;93m{}\x1b[0m", unsafe {
                core::str::from_utf8_unchecked(buf.as_mut_slice())
            });
        }
        60 => {
            panic!("sys_exit({})", a1);
        }
        _ => {
            unimplemented!();
        }
    }

    n
}

extern "C" {
    fn syscall_entry();
}

pub unsafe fn init() {
    wrmsr(
        msr::IA32_STAR,
        ((USER_CS32 as u64) << 48) | ((KERNEL_CS as u64) << 32),
    );
    wrmsr(msr::IA32_LSTAR, syscall_entry as *const u8 as u64);
    wrmsr(msr::IA32_FMASK, SYSCALL_RFLAGS_MASK);

    // RIP for compatibility mode. We don't support it for now.
    wrmsr(msr::IA32_CSTAR, 0);

    // Enable SYSCALL/SYSRET.
    wrmsr(msr::IA32_EFER, rdmsr(msr::IA32_EFER) | 1);
}

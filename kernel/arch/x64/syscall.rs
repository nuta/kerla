use core::unimplemented;

use crate::syscall::syscall::SyscallContext;

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
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    n: usize,
) -> isize {
    println!("syscall: n={}", n);
    let mut context = SyscallContext::new();
    context
        .dispatch(a1, a2, a3, a4, a5, a6, n)
        .unwrap_or_else(|err| -(err.errno() as isize))
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

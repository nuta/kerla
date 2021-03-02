use super::gdt::{KERNEL_CS, USER_CS32};
use x86::msr::{self, rdmsr, wrmsr};

// Clear IF bit to disable interrupts when we enter the syscall handler
// or an interrupt occurs before doing SWAPGS.
const SYSCALL_RFLAGS_MASK: u64 = 0x200;

#[allow(unused)]
#[no_mangle]
extern "C" fn x64_handle_syscall(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
    0
}

extern "C" {
    fn syscall_entry();
}

pub unsafe fn init() {
    wrmsr(
        msr::IA32_STAR,
        ((USER_CS32 as u64) << 48) | ((KERNEL_CS as u64) << 32),
    );
    wrmsr(msr::IA32_LSTAR, &syscall_entry as *const _ as u64);
    wrmsr(msr::IA32_FMASK, SYSCALL_RFLAGS_MASK);

    // RIP for compatibility mode. We don't support it for now.
    wrmsr(msr::IA32_CSTAR, 0);

    // Enable SYSCALL/SYSRET.
    wrmsr(msr::IA32_EFER, rdmsr(msr::IA32_EFER) | 1);
}

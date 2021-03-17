use super::apic::ack_interrupt;
use x86::current::rflags::{self, RFlags};

/// The interrupt stack frame.
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct InterruptFrame {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rdi: u64,
    error: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

use core::sync::atomic::{AtomicUsize, Ordering};
static TICKS: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
unsafe extern "C" fn x64_handle_interrupt(vec: u8, frame: *const InterruptFrame) {
    // FIXME: Check "Legacy replacement" mapping
    let is_timer = vec == 34;

    if !is_timer {
        println!(
            "interrupt({}): rip={:x}, rsp={:x}, err={:x}, cr2={:x}",
            vec,
            (*frame).rip,
            (*frame).rsp,
            (*frame).error,
            x86::controlregs::cr2()
        );
        panic!();
    }

    ack_interrupt();

    if is_timer {
        let value = TICKS.fetch_add(1, Ordering::Relaxed);
        if value % 20 == 0 {
            crate::thread::switch_thread();
        }
    }
}

pub unsafe fn disable_interrupt() {
    asm!("cli");
}

pub unsafe fn enable_interrupt() {
    asm!("sti");
}

pub fn is_interrupt_enabled() -> bool {
    unsafe { x86::current::rflags::read().contains(RFlags::FLAGS_IF) }
}

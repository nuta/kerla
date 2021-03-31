use super::{apic::ack_interrupt, PageFaultReason, UserVAddr};
use crate::{
    drivers::handle_irq,
    mm::page_fault::handle_page_fault,
    process::{switch, ProcessState},
};

use x86::{controlregs::cr2, current::rflags::RFlags};

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

extern "C" {
    fn usercopy1();
    fn usercopy2();
}

#[no_mangle]
unsafe extern "C" fn x64_handle_interrupt(vec: u8, frame: *const InterruptFrame) {
    let frame = &*frame;
    // FIXME: Check "Legacy replacement" mapping
    let is_timer = vec == 34;

    if !is_timer && vec != 14 && vec != 36 {
        println!(
            "interrupt({}): rip={:x}, rsp={:x}, err={:x}, cr2={:x}",
            vec,
            frame.rip,
            frame.rsp,
            frame.error,
            x86::controlregs::cr2()
        );
    }

    if is_timer {
        ack_interrupt();
        let value = TICKS.fetch_add(1, Ordering::Relaxed);
        if value % 20 == 0 {
            crate::process::switch(crate::process::ProcessState::Runnable);
        }
        return;
    }

    if vec == 36 {
        ack_interrupt();
        super::serial::irq_handler();
        return;
    }

    if vec == 14 {
        let reason = PageFaultReason::from_bits_truncate(frame.error as u32);

        // Panic if it's occurred in the kernel space.
        let occurred_in_user = reason.contains(PageFaultReason::CAUSED_BY_USER)
            || frame.rip == usercopy1 as *const u8 as u64
            || frame.rip == usercopy2 as *const u8 as u64;
        if !occurred_in_user {
            panic!(
                "page fault occurred in the kernel: rip={:x}, rsp={:x}, vaddr={:x}",
                frame.rip,
                frame.rsp,
                cr2()
            );
        }

        // Abort if the virtual address points to out of the user's address space.
        let unaligned_vaddr = match UserVAddr::new(cr2() as usize) {
            Ok(uvaddr) => uvaddr,
            Err(_) => {
                // TODO: Kill the current user process.
                todo!();
            }
        };

        handle_page_fault(unaligned_vaddr, reason);
        return;
    }

    if vec > 32 {
        handle_irq(vec - 32);
        ack_interrupt();
        return;
    }

    println!("WARN: unsupported interrupt vector");
    switch(ProcessState::Sleeping);
    unreachable!();
}

pub unsafe fn enable_interrupt() {
    asm!("sti");
}

pub fn is_interrupt_enabled() -> bool {
    unsafe { x86::current::rflags::read().contains(RFlags::FLAGS_IF) }
}

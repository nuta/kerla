use super::{apic::ack_interrupt, PageFaultReason, UserVAddr};
use crate::mm::page_fault::handle_page_fault;
use bitflags::bitflags;
use x86::{
    controlregs::cr2,
    current::rflags::{self, RFlags},
};

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
    fn usercopy();
}

#[no_mangle]
unsafe extern "C" fn x64_handle_interrupt(vec: u8, frame: *const InterruptFrame) {
    let frame = &*frame;
    // FIXME: Check "Legacy replacement" mapping
    let is_timer = vec == 34;

    if !is_timer {
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
            crate::process::switch();
        }
        return;
    }

    if vec == 14 {
        crate::printk::backtrace();
        let vaddr = unsafe { cr2() as usize };
        let reason = PageFaultReason::from_bits_truncate(frame.error as u32);

        // Panic if it's occurred in the kernel space.
        let occurred_in_user = reason.contains(PageFaultReason::CAUSED_BY_USER)
            || frame.rip == usercopy as *const u8 as u64;
        if !occurred_in_user {
            panic!(
                "page fault occurred in the kernel: rip={:x}, vaddr={:x}",
                frame.rip, vaddr
            );
        }

        // Abort if the virtual address points to out of the user's address space.
        let unaligned_vaddr = match UserVAddr::new(vaddr) {
            Ok(uvaddr) => uvaddr,
            Err(_) => {
                // TODO: Kill the current user process.
                todo!();
            }
        };

        handle_page_fault(unaligned_vaddr, reason);
        return;
    }

    todo!();
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

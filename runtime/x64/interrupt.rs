use crate::{address::UserVAddr, handler};

use super::{apic::ack_interrupt, ioapic::VECTOR_IRQ_BASE, serial::SERIAL_IRQ, PageFaultReason};
use x86::{
    controlregs::cr2,
    current::rflags::{self, RFlags},
    irq::*,
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

extern "C" {
    fn usercopy1();
    fn usercopy2();
    fn usercopy3();
}

#[no_mangle]
#[allow(unaligned_references)]
unsafe extern "C" fn x64_handle_interrupt(vec: u8, frame: *const InterruptFrame) {
    let frame = &*frame;

    // FIXME: Check "Legacy replacement" mapping
    const TIMER_IRQ: u8 = 0;
    const TIMER_IRQ2: u8 = 2;
    if vec != VECTOR_IRQ_BASE + TIMER_IRQ
        && vec != VECTOR_IRQ_BASE + TIMER_IRQ2
        && vec != 14
        && vec != 36
    {
        trace!(
            "interrupt({}): rip={:x}, rsp={:x}, err={:x}, cr2={:x}",
            vec,
            frame.rip,
            frame.rsp,
            frame.error,
            x86::controlregs::cr2()
        );
    }

    match vec {
        _ if vec >= VECTOR_IRQ_BASE => {
            ack_interrupt();

            let irq = vec - VECTOR_IRQ_BASE;
            match irq {
                TIMER_IRQ | TIMER_IRQ2 => {
                    handler().handle_timer_irq();
                }
                SERIAL_IRQ => {
                    super::serial::irq_handler();
                }
                _ => {
                    handler().handle_irq(irq);
                }
            }
        }
        DIVIDE_ERROR_VECTOR => {
            // TODO:
            todo!("unsupported exception: DIVIDE_ERROR");
        }
        DEBUG_VECTOR => {
            // TODO:
            todo!("unsupported exception: DEBUG");
        }
        NONMASKABLE_INTERRUPT_VECTOR => {
            // TODO:
            todo!("unsupported exception: NONMASKABLE_INTERRUPT");
        }
        BREAKPOINT_VECTOR => {
            // TODO:
            todo!("unsupported exception: BREAKPOINT");
        }
        OVERFLOW_VECTOR => {
            // TODO:
            todo!("unsupported exception: OVERFLOW");
        }
        BOUND_RANGE_EXCEEDED_VECTOR => {
            // TODO:
            todo!("unsupported exception: BOUND_RANGE_EXCEEDED");
        }
        INVALID_OPCODE_VECTOR => {
            // TODO:
            todo!("unsupported exception: INVALID_OPCODE");
        }
        DEVICE_NOT_AVAILABLE_VECTOR => {
            // TODO:
            todo!("unsupported exception: DEVICE_NOT_AVAILABLE");
        }
        DOUBLE_FAULT_VECTOR => {
            // TODO:
            todo!("unsupported exception: DOUBLE_FAULT");
        }
        COPROCESSOR_SEGMENT_OVERRUN_VECTOR => {
            // TODO:
            todo!("unsupported exception: COPROCESSOR_SEGMENT_OVERRUN");
        }
        INVALID_TSS_VECTOR => {
            // TODO:
            todo!("unsupported exception: INVALID_TSS");
        }
        SEGMENT_NOT_PRESENT_VECTOR => {
            // TODO:
            todo!("unsupported exception: SEGMENT_NOT_PRESENT");
        }
        STACK_SEGEMENT_FAULT_VECTOR => {
            // TODO:
            todo!("unsupported exception: STACK_SEGEMENT_FAULT");
        }
        GENERAL_PROTECTION_FAULT_VECTOR => {
            // TODO:
            todo!("unsupported exception: GENERAL_PROTECTION_FAULT");
        }
        PAGE_FAULT_VECTOR => {
            let reason = PageFaultReason::from_bits_truncate(frame.error as u32);

            // Panic if it's occurred in the kernel space.
            let occurred_in_user = reason.contains(PageFaultReason::CAUSED_BY_USER)
                || frame.rip == usercopy1 as *const u8 as u64
                || frame.rip == usercopy2 as *const u8 as u64
                || frame.rip == usercopy3 as *const u8 as u64;
            if !occurred_in_user {
                panic!(
                    "page fault occurred in the kernel: rip={:x}, rsp={:x}, vaddr={:x}",
                    frame.rip,
                    frame.rsp,
                    cr2()
                );
            }

            // Abort if the virtual address points to out of the user's address space.
            let unaligned_vaddr = UserVAddr::new(cr2() as usize);
            handler().handle_page_fault(unaligned_vaddr, frame.rip as usize, reason);
        }
        X87_FPU_VECTOR => {
            // TODO:
            todo!("unsupported exception: X87_FPU");
        }
        ALIGNMENT_CHECK_VECTOR => {
            // TODO:
            todo!("unsupported exception: ALIGNMENT_CHECK");
        }
        MACHINE_CHECK_VECTOR => {
            // TODO:
            todo!("unsupported exception: MACHINE_CHECK");
        }
        SIMD_FLOATING_POINT_VECTOR => {
            // TODO:
            todo!("unsupported exception: SIMD_FLOATING_POINT");
        }
        VIRTUALIZATION_VECTOR => {
            // TODO:
            todo!("unsupported exception: VIRTUALIZATION");
        }
        _ => {
            panic!("unexpected interrupt: vec={}", vec);
        }
    }
}

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

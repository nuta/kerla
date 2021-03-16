use super::apic::ack_interrupt;

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

#[no_mangle]
unsafe extern "C" fn x64_handle_interrupt(vec: u8, frame: *const InterruptFrame) {
    // FIXME: Check "Legacy replacement" mapping
    let is_timer = vec == 34;

    if !is_timer {
        println!(
            "interrupt({}): rip={:x}, rsp={:x}, err={:x}",
            vec,
            (*frame).rip,
            (*frame).rsp,
            (*frame).error
        );
    }

    ack_interrupt();
}

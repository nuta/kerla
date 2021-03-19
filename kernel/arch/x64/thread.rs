use super::{
    address::VAddr,
    gdt::{USER_CS64, USER_DS},
    tss::{IST_RSP0, TSS},
    UserVAddr, KERNEL_STACK_SIZE,
};
use super::{cpu_local::cpu_local_head, gdt::USER_RPL};
use crate::mm::page_allocator::alloc_pages;
use x86::bits64::segmentation::wrgsbase;

#[repr(C, packed)]
pub struct Thread {
    rsp: u64,
    interrupt_stack: VAddr,
    syscall_stack: VAddr,
}

extern "C" {
    fn kthread_entry();
    fn userland_entry();
    fn do_switch_thread(prev_rsp: *const u64, next_rsp: *const u64);
}

unsafe fn push_stack(mut rsp: *mut u64, value: u64) -> *mut u64 {
    rsp = rsp.sub(1);
    rsp.write(value);
    rsp
}

impl Thread {
    pub fn new_kthread(ip: VAddr, sp: VAddr) -> Thread {
        let interrupt_stack = alloc_pages(1)
            .expect("failed to allocate kernel stack")
            .as_vaddr();
        let syscall_stack = alloc_pages(1)
            .expect("failed to allocate kernel stack")
            .as_vaddr();

        let rsp = unsafe {
            let mut rsp: *mut u64 = sp.as_mut_ptr();

            // Registers to be restored in kthread_entry().
            rsp = push_stack(rsp, ip.value() as u64); // The entry point.

            // Registers to be restored in do_switch_thread().
            rsp = push_stack(rsp, kthread_entry as *const u8 as u64); // RIP.
            rsp = push_stack(rsp, 0); // Initial RBP.
            rsp = push_stack(rsp, 0); // Initial RBX.
            rsp = push_stack(rsp, 0); // Initial R12.
            rsp = push_stack(rsp, 0); // Initial R13.
            rsp = push_stack(rsp, 0); // Initial R14.
            rsp = push_stack(rsp, 0); // Initial R15.
            rsp = push_stack(rsp, 0x02); // RFLAGS (interrupts disabled).

            rsp
        };

        Thread {
            rsp: rsp as u64,
            interrupt_stack,
            syscall_stack,
        }
    }

    pub fn new_user_thread(ip: UserVAddr, sp: UserVAddr, kernel_sp: VAddr) -> Thread {
        let interrupt_stack = alloc_pages(1)
            .expect("failed to allocate kernel stack")
            .as_vaddr();
        let syscall_stack = alloc_pages(1)
            .expect("failed to allocate kernel stack")
            .as_vaddr();

        let rsp = unsafe {
            let mut rsp: *mut u64 = kernel_sp.as_mut_ptr();

            // Registers to be restored by IRET.
            rsp = push_stack(rsp, (USER_DS | USER_RPL) as u64); // SS
            rsp = push_stack(rsp, sp.value() as u64); // user RSP
            rsp = push_stack(rsp, 0x202); // RFLAGS (interrupts enabled).
            rsp = push_stack(rsp, (USER_CS64 | USER_RPL) as u64); // CS
            rsp = push_stack(rsp, ip.value() as u64); // RIP

            // Registers to be restored in do_switch_thread().
            rsp = push_stack(rsp, userland_entry as *const u8 as u64); // RIP.
            rsp = push_stack(rsp, 0); // Initial RBP.
            rsp = push_stack(rsp, 0); // Initial RBX.
            rsp = push_stack(rsp, 0); // Initial R12.
            rsp = push_stack(rsp, 0); // Initial R13.
            rsp = push_stack(rsp, 0); // Initial R14.
            rsp = push_stack(rsp, 0); // Initial R15.
            rsp = push_stack(rsp, 0x02); // RFLAGS (interrupts disabled).

            rsp
        };

        Thread {
            rsp: rsp as u64,
            interrupt_stack,
            syscall_stack,
        }
    }

    pub fn new_idle_thread() -> Thread {
        let interrupt_stack = alloc_pages(1)
            .expect("failed to allocate kernel stack")
            .as_vaddr();
        let syscall_stack = alloc_pages(1)
            .expect("failed to allocate kernel stack")
            .as_vaddr();

        Thread {
            rsp: 0,
            interrupt_stack,
            syscall_stack,
        }
    }
}

pub fn switch_thread(prev: &mut Thread, next: &mut Thread) {
    let head = cpu_local_head();

    // Switch the kernel stack.
    head.rsp0 = (next.syscall_stack.value() + KERNEL_STACK_SIZE) as u64;
    TSS.as_mut()
        .set_rsp0((next.interrupt_stack.value() + KERNEL_STACK_SIZE) as u64);

    // Fill an invalid value for now: must be initialized in interrupt handlers.
    head.rsp3 = 0xbaad_5a5a_5b5b_baad;

    unsafe {
        do_switch_thread(&mut prev.rsp as *mut u64, &mut next.rsp as *mut u64);
    }
}

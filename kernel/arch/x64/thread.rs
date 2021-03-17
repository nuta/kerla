use super::address::VAddr;
use super::cpu_local::cpu_local_head;
use x86::bits64::segmentation::wrgsbase;

#[repr(C, packed)]
pub struct Thread {
    rsp: u64,
}

extern "C" {
    fn kthread_entry();
    fn do_switch_thread(prev_rsp: *const u64, next_rsp: *const u64);
}

unsafe fn push_stack(mut rsp: *mut u64, value: u64) -> *mut u64 {
    rsp = rsp.sub(1);
    rsp.write(value);
    rsp
}

impl Thread {
    pub fn new_kthread(ip: VAddr, sp: VAddr) -> Thread {
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

        Thread { rsp: rsp as u64 }
    }

    pub fn new_idle_thread() -> Thread {
        Thread { rsp: 0 }
    }
}

pub fn switch_thread(prev: &mut Thread, next: &mut Thread) {
    let head = cpu_local_head();
    head.rsp0 = 0x000001111beef0001;
    head.rsp3 = 0x000001111beee0002;
    unsafe {
        do_switch_thread(&mut prev.rsp as *mut u64, &mut next.rsp as *mut u64);
    }
}

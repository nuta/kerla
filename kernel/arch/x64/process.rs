use core::cell::UnsafeCell;

use crate::result::Result;
use crate::{arch::KERNEL_STACK_SIZE, process::signal::Signal};
use crossbeam::atomic::AtomicCell;
use kerla_runtime::address::{UserVAddr, VAddr};
use kerla_runtime::page_allocator::{alloc_pages_owned, OwnedPages};
use kerla_runtime::{
    arch::x64_specific::{cpu_local_head, TSS, USER_CS64, USER_DS, USER_RPL},
    arch::PtRegs,
    arch::PAGE_SIZE,
    page_allocator::AllocPageFlags,
};
use x86::current::segmentation::wrfsbase;

pub struct Process {
    rsp: UnsafeCell<u64>,
    pub(super) fsbase: AtomicCell<u64>,
    pub(super) xsave_area: Option<OwnedPages>,
    // This appears dead, but really we're keeping the pages referenced from the
    // rsp from being dropped until the Process is dropped.
    #[allow(dead_code)]
    kernel_stack: OwnedPages,
    // FIXME: Do we really need these stacks?
    interrupt_stack: OwnedPages,
    syscall_stack: OwnedPages,
}

unsafe impl Sync for Process {}

extern "C" {
    fn kthread_entry();
    fn userland_entry();
    fn forked_child_entry();
    fn do_switch_thread(prev_rsp: *const u64, next_rsp: *const u64);
}

unsafe fn push_stack(mut rsp: *mut u64, value: u64) -> *mut u64 {
    rsp = rsp.sub(1);
    rsp.write(value);
    rsp
}

impl Process {
    #[allow(unused)]
    pub fn new_kthread(ip: VAddr, sp: VAddr) -> Process {
        let interrupt_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate interrupt stack");
        let syscall_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate syscall stack");
        let kernel_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate kernel stack");

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

        Process {
            rsp: UnsafeCell::new(rsp as u64),
            fsbase: AtomicCell::new(0),
            xsave_area: None,
            interrupt_stack,
            syscall_stack,
            kernel_stack,
        }
    }

    pub fn new_user_thread(ip: UserVAddr, sp: UserVAddr) -> Process {
        let kernel_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate kernel stack");
        let interrupt_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate kernel stack");
        let syscall_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate kernel stack");
        let xsave_area =
            alloc_pages_owned(1, AllocPageFlags::KERNEL).expect("failed to allocate xsave area");

        let rsp = unsafe {
            let kernel_sp = kernel_stack.as_vaddr().add(KERNEL_STACK_SIZE);
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

        Process {
            rsp: UnsafeCell::new(rsp as u64),
            fsbase: AtomicCell::new(0),
            xsave_area: Some(xsave_area),
            interrupt_stack,
            syscall_stack,
            kernel_stack,
        }
    }

    pub fn new_idle_thread() -> Process {
        let interrupt_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate kernel stack");
        let syscall_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate kernel stack");
        let kernel_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocat kernel stack");

        Process {
            rsp: UnsafeCell::new(0),
            fsbase: AtomicCell::new(0),
            xsave_area: None,
            interrupt_stack,
            syscall_stack,
            kernel_stack,
        }
    }

    pub fn fork(&self, frame: &PtRegs) -> Result<Process> {
        let xsave_area =
            alloc_pages_owned(1, AllocPageFlags::KERNEL).expect("failed to allocate xsave area");
        let kernel_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed to allocate kernel stack");
        let rsp = unsafe {
            let kernel_sp = kernel_stack.as_vaddr().add(KERNEL_STACK_SIZE);
            let mut rsp: *mut u64 = kernel_sp.as_mut_ptr();

            // Registers to be restored by IRET.
            rsp = push_stack(rsp, (USER_DS | USER_RPL) as u64); // SS
            rsp = push_stack(rsp, frame.rsp); // user RSP
            rsp = push_stack(rsp, frame.rflags); // user RFLAGS.
            rsp = push_stack(rsp, (USER_CS64 | USER_RPL) as u64); // CS
            rsp = push_stack(rsp, frame.rip); // user RIP

            // Registers to be restored in forked_child_entry,
            rsp = push_stack(rsp, frame.rflags); // user R11
            rsp = push_stack(rsp, frame.rip); // user RCX
            rsp = push_stack(rsp, frame.r10);
            rsp = push_stack(rsp, frame.r9);
            rsp = push_stack(rsp, frame.r8);
            rsp = push_stack(rsp, frame.rsi);
            rsp = push_stack(rsp, frame.rdi);
            rsp = push_stack(rsp, frame.rdx);

            // Registers to be restored in do_switch_thread().
            rsp = push_stack(rsp, forked_child_entry as *const u8 as u64); // RIP.
            rsp = push_stack(rsp, frame.rbp); // UserRBP.
            rsp = push_stack(rsp, frame.rbx); // UserRBX.
            rsp = push_stack(rsp, frame.r12); // UserR12.
            rsp = push_stack(rsp, frame.r13); // UserR13.
            rsp = push_stack(rsp, frame.r14); // UserR14.
            rsp = push_stack(rsp, frame.r15); // UserR15.
            rsp = push_stack(rsp, 0x02); // RFLAGS (interrupts disabled).

            rsp
        };

        let interrupt_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed allocate interrupt stack");
        let syscall_stack = alloc_pages_owned(
            KERNEL_STACK_SIZE / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::DIRTY_OK,
        )
        .expect("failed allocate syscall stack");

        Ok(Process {
            rsp: UnsafeCell::new(rsp as u64),
            fsbase: AtomicCell::new(self.fsbase.load()),
            xsave_area: Some(xsave_area),
            interrupt_stack,
            syscall_stack,
            kernel_stack,
        })
    }

    pub fn setup_execve_stack(
        &self,
        frame: &mut PtRegs,
        ip: UserVAddr,
        user_sp: UserVAddr,
    ) -> Result<()> {
        frame.rip = ip.as_isize() as u64;
        frame.rsp = user_sp.as_isize() as u64;
        Ok(())
    }

    pub unsafe fn setup_signal_stack(
        &self,
        frame: &mut PtRegs,
        signal: Signal,
        sa_handler: UserVAddr,
    ) -> Result<()> {
        const TRAMPOLINE: &[u8] = &[
            0xb8, 0x0f, 0x00, 0x00, 0x00, // mov eax, 15
            0x0f, 0x05, // syscall
            0x90, // nop (for alignment)
        ];

        fn push_to_user_stack(rsp: UserVAddr, value: u64) -> Result<UserVAddr> {
            let rsp = rsp.sub(8);
            rsp.write::<u64>(&value)?;
            Ok(rsp)
        }

        // Prepare the sigreturn stack in the userspace.
        let mut user_rsp = UserVAddr::new_nonnull(frame.rsp as usize)?;

        // Avoid corrupting the red zone.
        user_rsp = user_rsp.sub(128);

        // Copy the trampoline code.
        user_rsp = user_rsp.sub(TRAMPOLINE.len());
        let trampoline_rip = user_rsp;
        user_rsp.write_bytes(TRAMPOLINE)?;
        user_rsp = push_to_user_stack(user_rsp, trampoline_rip.as_isize() as u64)?;

        frame.rip = sa_handler.as_isize() as u64;
        frame.rsp = user_rsp.as_isize() as u64;
        frame.rdi = signal as u64; // int signal
        frame.rsi = 0; // siginfo_t *siginfo
        frame.rdx = 0; // void *ctx

        Ok(())
    }

    pub fn setup_sigreturn_stack(&self, current_frame: &mut PtRegs, signaled_frame: &PtRegs) {
        *current_frame = *signaled_frame;
    }
}

pub fn switch_thread(prev: &Process, next: &Process) {
    let head = cpu_local_head();

    // Switch the kernel stack.
    head.rsp0 = (next.syscall_stack.as_vaddr().value() + KERNEL_STACK_SIZE) as u64;
    TSS.as_mut()
        .set_rsp0((next.interrupt_stack.as_vaddr().value() + KERNEL_STACK_SIZE) as u64);

    // Save and restore the XSAVE area (i.e. XMM/YMM registrers).
    unsafe {
        use core::arch::x86_64::{_xrstor64, _xsave64};

        let xsave_mask = x86::controlregs::xcr0().bits();
        if let Some(xsave_area) = prev.xsave_area.as_ref() {
            _xsave64(xsave_area.as_mut_ptr(), xsave_mask);
        }
        if let Some(xsave_area) = next.xsave_area.as_ref() {
            _xrstor64(xsave_area.as_mut_ptr(), xsave_mask);
        }
    }

    // Fill an invalid value for now: must be initialized in interrupt handlers.
    head.rsp3 = 0xbaad_5a5a_5b5b_baad;

    unsafe {
        wrfsbase(next.fsbase.load());
        do_switch_thread(prev.rsp.get(), next.rsp.get());
    }
}

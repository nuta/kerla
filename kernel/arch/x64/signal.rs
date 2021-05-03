use super::{SpinLock, SyscallFrame, Thread, UserVAddr};

use crate::process::signal::Signal;
use crate::{mm::vm::Vm, prelude::*, process::current_process};

const TRAMPOLINE: &[u8] = &[
    0xb8, 0x0f, 0x00, 0x00, 0x00, // mov eax, 15
    0x0f, 0x05, // syscall
    0x90, // nop (for alignment)
];

#[must_use]
fn push_to_user_stack(rsp: UserVAddr, value: u64) -> Result<UserVAddr> {
    let rsp = rsp.sub(8)?;
    rsp.write::<u64>(&value)?;
    Ok(rsp)
}

pub fn setup_signal_handler_stack(
    thread: &SpinLock<Thread>,
    vm: &SpinLock<Vm>,
    frame: &SyscallFrame,
    signal: Signal,
    sa_handler: UserVAddr,
    is_current_process: bool,
) -> Result<()> {
    // Prepare the sigreturn stack in the userspace.
    let user_rsp = {
        // `thread` can be an arbitrary process. Temporarily switch the page
        // table to access its virtual memory space.
        vm.lock().page_table().switch();

        let mut user_rsp = UserVAddr::new_nonnull(frame.rsp as usize)?;

        // Avoid corrupting the red zone.
        user_rsp = user_rsp.sub(128)?;

        // Copy the trampoline code.
        user_rsp = user_rsp.sub(TRAMPOLINE.len())?;
        let trampoline_rip = user_rsp;
        user_rsp.write_bytes(TRAMPOLINE)?;

        user_rsp = push_to_user_stack(user_rsp, trampoline_rip.as_isize() as u64)?;

        // Restore the current process's page table.
        if let Some(current_vm) = current_process().vm.as_ref() {
            current_vm.lock().page_table().switch();
        }

        user_rsp
    };

    let arg1 = signal as u64; // int signal
    let arg2 = 0; // siginfo_t *siginfo
    let arg3 = 0; // void *ctx

    unsafe {
        Thread::set_signal_entry(
            thread.lock(),
            sa_handler.as_isize() as u64,
            user_rsp.as_isize() as u64,
            arg1,
            arg2,
            arg3,
            is_current_process,
        );
    }

    Ok(())
}

pub fn restore_signaled_stack(current_frame: &mut SyscallFrame, signaled_frame: &SyscallFrame) {
    *current_frame = *signaled_frame;
}

use kerla_runtime::arch::halt;

use crate::{ctypes::c_int, result::Result, syscalls::SyscallHandler};

impl<'a> SyscallHandler<'a> {
    pub fn sys_reboot(&mut self, _magic: c_int, _magic2: c_int, _arg: usize) -> Result<isize> {
        info!("Halting the system by reboot(2)");
        halt();
    }
}

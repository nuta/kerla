use crate::{
    ctypes::c_int,
    syscalls::SyscallHandler,
    {arch::halt, result::Result},
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_reboot(&mut self, _magic: c_int, _magic2: c_int, _arg: usize) -> Result<isize> {
        info!("Halting the system by reboot(2)");
        halt();
    }
}

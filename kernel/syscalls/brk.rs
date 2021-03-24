use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl SyscallDispatcher {
    pub fn sys_brk(&mut self, new_heap_end: UserVAddr) -> Result<isize> {
        let mut vm = current_process().vm();
        if !new_heap_end.is_null() {
            vm.expand_heap_to(new_heap_end)?;
        }
        Ok(vm.heap_end().value() as isize)
    }
}

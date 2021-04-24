use crate::{arch::UserVAddr, result::Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_brk(&mut self, new_heap_end: Option<UserVAddr>) -> Result<isize> {
        let mut vm = current_process().vm();
        if let Some(new_heap_end) = new_heap_end {
            vm.expand_heap_to(new_heap_end)?;
        }
        Ok(vm.heap_end().value() as isize)
    }
}

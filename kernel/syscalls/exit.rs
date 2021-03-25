use crate::{
    process::{current_process, switch},
    syscalls::SyscallDispatcher,
};

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_exit(&mut self, _status: i32) -> ! {
        current_process().exit();
        switch(crate::process::ProcessState::Sleeping);
        todo!()
    }
}

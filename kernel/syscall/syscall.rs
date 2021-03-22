use crate::{
    arch::UserVAddr,
    fs::opened_file::Fd,
    result::{Errno, Error, Result},
};

const SYS_WRITE: usize = 1;
const SYS_EXIT: usize = 60;

pub struct SyscallContext {}

impl SyscallContext {
    pub fn new() -> SyscallContext {
        SyscallContext {}
    }

    pub fn dispatch(
        &mut self,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        a5: usize,
        a6: usize,
        n: usize,
    ) -> Result<isize> {
        match n {
            SYS_WRITE => self.sys_write(Fd::new(a1 as i32), UserVAddr::new(a2)?, a3),
            _ => Err(Error::new(Errno::ENOSYS)),
        }
    }
}

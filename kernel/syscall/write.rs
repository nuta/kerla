use crate::{
    arch::UserVAddr,
    fs::opened_file::Fd,
    result::{Errno, Error, Result},
};
use crate::{process::current_process, syscall::syscall::SyscallContext};

impl SyscallContext {
    pub fn sys_write(&mut self, fd: Fd, uaddr: UserVAddr, len: usize) -> Result<isize> {
        let mut buf = vec![0; len]; // TODO: deny too long len
        uaddr.read_bytes(&mut buf);
        let current = current_process().opened_files.lock();
        let open_file = current.get(fd)?;
        let file = open_file.as_file()?;
        file.write(open_file.pos(), buf.as_slice())
            .map(|len| len as isize /* FIXME: */)
    }
}

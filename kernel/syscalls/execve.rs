use crate::fs::path::Path;
use crate::prelude::*;
use crate::process::Process;
use crate::user_buffer::UserCStr;
use crate::{process::current_process, syscalls::SyscallHandler};
use core::mem::size_of;
use kerla_runtime::address::UserVAddr;

const ARG_MAX: usize = 512;
const ARG_LEN_MAX: usize = 4096;
const ENV_MAX: usize = 512;
const ENV_LEN_MAX: usize = 4096;

impl<'a> SyscallHandler<'a> {
    pub fn sys_execve(
        &mut self,
        path: &Path,
        argv_uaddr: UserVAddr,
        envp_uaddr: UserVAddr,
    ) -> Result<isize> {
        let current = current_process();
        let executable = current.root_fs().lock().lookup_path(path, true)?;

        let mut argv = Vec::new();
        for i in 0..ARG_MAX {
            let ptr = argv_uaddr.add(i * size_of::<usize>());
            match UserVAddr::new(ptr.read::<usize>()?) {
                Some(str_ptr) => argv.push(UserCStr::new(str_ptr, ARG_LEN_MAX)?),
                None => break,
            }
        }

        let mut envp = Vec::new();
        for i in 0..ENV_MAX {
            let ptr = envp_uaddr.add(i * size_of::<usize>());
            match UserVAddr::new(ptr.read::<usize>()?) {
                Some(str_ptr) => envp.push(UserCStr::new(str_ptr, ENV_LEN_MAX)?),
                None => break,
            }
        }

        let argv_slice: Vec<&[u8]> = argv.as_slice().iter().map(|s| s.as_bytes()).collect();
        let envp_slice: Vec<&[u8]> = envp.as_slice().iter().map(|s| s.as_bytes()).collect();
        Process::execve(self.frame, executable, &argv_slice, &envp_slice)?;
        Ok(0)
    }
}

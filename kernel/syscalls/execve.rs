use super::dispatcher::UserCStr;
use crate::fs::path::Path;
use crate::process::{switch, ProcessState};
use crate::{arch::UserVAddr, result::Result};
use crate::{
    process::{current_process, execve},
    syscalls::SyscallDispatcher,
};
use alloc::vec::Vec;
use core::mem::size_of;

const ARG_MAX: usize = 512;
const ARG_LEN_MAX: usize = 4096;
const ENV_MAX: usize = 512;
const ENV_LEN_MAX: usize = 4096;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_execve(
        &mut self,
        path: &Path,
        argv_uaddr: UserVAddr,
        envp_uaddr: UserVAddr,
    ) -> Result<isize> {
        let current = current_process();
        let executable = current.root_fs.lock().lookup_file(path)?;

        let mut argv = Vec::new();
        for i in 0..ARG_MAX {
            let ptr = argv_uaddr.add(i * size_of::<usize>())?;
            let str_ptr = ptr.read::<UserVAddr>()?;
            if str_ptr.is_null() {
                break;
            }
            argv.push(UserCStr::new(str_ptr, ARG_LEN_MAX)?);
        }

        let mut envp = Vec::new();
        for i in 0..ENV_MAX {
            let ptr = envp_uaddr.add(i * size_of::<usize>())?;
            let str_ptr = ptr.read::<UserVAddr>()?;
            if str_ptr.is_null() {
                break;
            }

            envp.push(UserCStr::new(str_ptr, ENV_LEN_MAX)?);
        }

        let argv_slice: Vec<&[u8]> = argv.as_slice().iter().map(|s| s.as_bytes()).collect();
        let envp_slice: Vec<&[u8]> = envp.as_slice().iter().map(|s| s.as_bytes()).collect();

        execve(
            current.parent.clone(),
            current.pid,
            executable,
            &argv_slice,
            &envp_slice,
            current.root_fs.clone(),
            current.opened_files.clone(),
        )?;

        current_process().set_state(ProcessState::Execved);
        switch();
        unreachable!();
    }
}

use crate::prelude::*;
use crate::syscalls::SyscallHandler;
use crate::{
    ctypes::*
};
use kerla_runtime::address::UserVAddr;
use bitflags::bitflags;


bitflags! {
    /// Flags used by [`clone()`] system call
    /// Source: `/usr/include/linux/sched.h`
    ///
    /// [`clone()`]: https://linux.die.net/man/2/clone
    pub struct CloneFlags : c_uint {
        const CLONE_VM             = 0x00000100;     // set if VM shared between processes
        const CLONE_FS             = 0x00000200;     // set if fs info shared between processes
        const CLONE_FILES          = 0x00000400;     // set if open files shared between processes
        const CLONE_SIGHAND        = 0x00000800;     // set if signal handlers and blocked signals shared
        const CLONE_PIDFD          = 0x00001000;     // set if a pidfd should be placed in parent
        const CLONE_PTRACE         = 0x00002000;     // set if we want to let tracing continue on the child too
        const CLONE_VFORK          = 0x00004000;     // set if the parent wants the child to wake it up on mm_release
        const CLONE_PARENT         = 0x00008000;     // set if we want to have the same parent as the cloner
        const CLONE_THREAD         = 0x00010000;     // Same thread group?
        const CLONE_NEWNS          = 0x00020000;     // New mount namespace group
        const CLONE_SYSVSEM        = 0x00040000;     // share system V SEM_UNDO semantics
        const CLONE_SETTLS         = 0x00080000;     // create a new TLS for the child
        const CLONE_PARENT_SETTID  = 0x00100000;     // set the TID in the parent
        const CLONE_CHILD_CLEARTID = 0x00200000;     // clear the TID in the child
        const CLONE_DETACHED       = 0x00400000;     // Unused, ignored
        const CLONE_UNTRACED       = 0x00800000;     // set if the tracing process can't force CLONE_PTRACE on this clone
        const CLONE_CHILD_SETTID   = 0x01000000;     // set the TID in the child
        const CLONE_NEWCGROUP      = 0x02000000;     // New cgroup namespace
        const CLONE_NEWUTS         = 0x04000000;     // New utsname namespace
        const CLONE_NEWIPC         = 0x08000000;     // New ipc namespace
        const CLONE_NEWUSER        = 0x10000000;     // New user namespace
        const CLONE_NEWPID         = 0x20000000;     // New pid namespace
        const CLONE_NEWNET         = 0x40000000;     // New network namespace
        const CLONE_IO             = 0x80000000;     // Clone io context
    }
}

#[repr(C, align(8))]
pub struct CloneArgs {
    flags:          u64,
    pidfd:          u64,
    child_tid:      u64,
    parent_tid:     u64,
    exit_signal:    u64,
    stack:          u64,
    stack_size:     u64,
    tls:            u64,
    set_tid:        u64,
    set_tid_size:   u64,
    cgroup:         u64,
}

/// Internal structure for `clone3()` system call
/// Modeled on `kernel_clone_args` from `include/linux/sched/task.h`
pub(crate) struct KernelCloneArgs {
    flags: CloneFlags,
    pidfd: Option<UserVAddr>,        /// Address to store pidfd into
    child_tid: Option<UserVAddr>,    /// Address to store child TID into or read futex address
    parent_tid: Option<UserVAddr>,   /// Address to store parent TID into...
    exit_signal: c_int,              /// Exit signal; TODO: I'd suggest enum for signal IDs for type safety
    stack: UserVAddr,
    stack_size: usize,
    tls: UserVAddr,
    set_tid: Option<UserVAddr>,      /// Handles TID array, requires namespace support
    set_tid_size: usize,             /// Number of elements in set_tid
    cgroup: c_int,
    io_thread: c_int,
    //struct cgroup *cgrp; // TODO: Kerla don't have support for cgroups yet
    //struct css_set *cset; // cgroup sets TODO: Kerla don't have support for cgroups yet
}

impl From<CloneArgs> for KernelCloneArgs {
    fn from(item: CloneArgs) -> Self {
        KernelCloneArgs {
            flags: CloneFlags::from_bits_truncate(item.flags as u32),
            pidfd: UserVAddr::new_nonnull(item.pidfd as usize).map_or(None, |v| Some(v)),
            child_tid: UserVAddr::new_nonnull(item.child_tid as usize).map_or(None, |v| Some(v)),
            parent_tid: UserVAddr::new_nonnull(item.parent_tid as usize).map_or(None, |v| Some(v)),
            exit_signal: item.exit_signal as c_int, // TODO: we need nicer signal type internally
            stack: unsafe { UserVAddr::new_unchecked(item.stack as usize) }, // TODO: this might be a problem - I struggle with distinction which type is output and which is input, damn C ABI
            stack_size: item.stack_size as usize,
            tls: unsafe { UserVAddr::new_unchecked(item.tls as usize) },
            set_tid: UserVAddr::new_nonnull(item.set_tid as usize).map_or(None, |v| Some(v)),
            set_tid_size: item.set_tid_size as usize,
            cgroup: item.cgroup as i32,
            io_thread: 0, // TODO: find out what's that!
        }
    }
}

impl<'a> SyscallHandler<'a> {
    // TODO: place `fork()` here either!
    pub fn _sys_fork(&mut self) -> Result<isize> {
        // struct kernel_clone_args args = {
        //     .exit_signal = SIGCHLD,
        // };

        // return kernel_clone(&args);
        Err(Error::new(Errno::ENOSYS))
    }

    pub fn sys_vfork(&mut self) -> Result<isize> {
        // struct kernel_clone_args args = {
        //     .flags          = CLONE_VFORK | CLONE_VM,
        //     .exit_signal    = SIGCHLD,
        // };

        // return kernel_clone(&args);
        Err(Error::new(Errno::ENOSYS))
    }

    pub fn sys_clone(&mut self, flags: CloneFlags, child_stack: Option<UserVAddr>,
        parent_tid: Option<UserVAddr>, child_tid: Option<UserVAddr>,
        tls: Option<UserVAddr>,
    ) -> Result<isize> {
        // TODO: implement

        if let Some(ptid_vaddr) = parent_tid {
            let ptid = 0i32;
            ptid_vaddr.write(&ptid);
        };

        if let Some(ctid_vaddr) = child_tid {
            let ctid = 0i32;
            ctid_vaddr.write(&ctid);
        };

        Err(Error::new(Errno::ENOSYS))
    }

    pub fn sys_clone3(&mut self, uargs: &CloneArgs, size: usize) -> Result<isize> {
        Err(Error::new(Errno::ENOSYS))
    }
}

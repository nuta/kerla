use super::*;
use crate::{
    arch::{self, disable_interrupt, enable_interrupt, is_interrupt_enabled, SpinLock, VAddr},
    elf::Elf,
    fs::initramfs::INITRAM_FS,
    fs::mount::RootFs,
    fs::opened_file,
    fs::path::Path,
    fs::{
        devfs::DEV_FS,
        inode::{FileLike, INode},
        opened_file::*,
        stat::Stat,
    },
    mm::{
        page_allocator::alloc_pages,
        vm::{Vm, VmAreaType},
    },
    result::{Errno, Error, ErrorExt, Result},
};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::{UserVAddr, KERNEL_STACK_SIZE, PAGE_SIZE, USER_STACK_TOP};
use arrayvec::ArrayVec;
use core::cmp::max;
use core::mem::{self, size_of, size_of_val};
use core::sync::atomic::{AtomicI32, Ordering};
use goblin::elf64::program_header::PT_LOAD;
use opened_file::OpenedFileTable;
use penguin_utils::once::Once;
use penguin_utils::{alignment::align_up, lazy::Lazy};

fn push_bytes_to_stack(sp: &mut VAddr, stack_bottom: VAddr, buf: &[u8]) -> Result<()> {
    if sp.sub(buf.len()) < stack_bottom {
        return Err(Error::with_message(Errno::E2BIG, "too big argvp/envp/auxv"));
    }

    *sp = sp.sub(buf.len());
    sp.write_bytes(buf);
    Ok(())
}

fn push_usize_to_stack(sp: &mut VAddr, stack_bottom: VAddr, value: usize) -> Result<()> {
    if cfg!(target_endian = "big") {
        push_bytes_to_stack(sp, stack_bottom, &value.to_be_bytes())?;
    } else {
        push_bytes_to_stack(sp, stack_bottom, &value.to_le_bytes())?;
    }

    Ok(())
}

#[repr(usize)]
pub enum Auxv {
    Null,
}

fn push_auxv_entry_to_stack(sp: &mut VAddr, stack_bottom: VAddr, auxv: &Auxv) -> Result<()> {
    let (auxv_type, value) = match auxv {
        Auxv::Null => (0, 0),
    };

    push_usize_to_stack(sp, stack_bottom, value)?;
    push_usize_to_stack(sp, stack_bottom, auxv_type)?;
    Ok(())
}

pub(super) fn estimate_user_init_stack_size(
    argv: &[&[u8]],
    envp: &[&[u8]],
    auxv: &[Auxv],
) -> usize {
    let str_len = align_up(
        argv.iter().fold(0, |l, arg| l + arg.len() + 1)
            + envp.iter().fold(0, |l, env| l + env.len() + 1),
        size_of::<usize>(),
    );

    let aux_data_len = auxv.iter().fold(0, |l, aux| {
        l + match aux {
            Auxv::Null => 0,
        }
    });

    let ptrs_len =
        (2 * (1 + auxv.len()) + argv.len() + 1 + envp.len() + 1 + 1) * size_of::<usize>();

    str_len + aux_data_len + ptrs_len
}

/// Initializes a user stack. See "Initial Process Stack" in <https://uclibc.org/docs/psABI-x86_64.pdf>.
pub(super) fn init_user_stack(
    user_stack_top: UserVAddr,
    stack_top: VAddr,
    stack_bottom: VAddr,
    argv: &[&[u8]],
    envp: &[&[u8]],
    auxv: &[Auxv],
) -> Result<UserVAddr> {
    let mut sp = stack_top;
    let kernel_sp_to_user_sp = |sp: VAddr| {
        let offset = stack_top.value() - sp.value();
        user_stack_top.sub(offset)
    };

    // Write envp strings.
    let mut envp_ptrs = Vec::with_capacity(argv.len());
    for env in envp {
        push_bytes_to_stack(&mut sp, stack_bottom, &[0]);
        push_bytes_to_stack(&mut sp, stack_bottom, env);
        envp_ptrs.push(kernel_sp_to_user_sp(sp)?);
    }

    // Write argv strings.
    let mut argv_ptrs = Vec::with_capacity(argv.len());
    for arg in argv {
        push_bytes_to_stack(&mut sp, stack_bottom, &[0]);
        push_bytes_to_stack(&mut sp, stack_bottom, arg);
        argv_ptrs.push(kernel_sp_to_user_sp(sp)?);
    }

    // The length of the string table wrote above could be unaligned.
    sp.align_down(size_of::<usize>());

    // Push auxiliary vector entries.
    push_auxv_entry_to_stack(&mut sp, stack_bottom, &Auxv::Null);
    for aux in auxv {
        push_auxv_entry_to_stack(&mut sp, stack_bottom, aux);
    }

    // Push environment pointers (`const char **envp`).
    push_usize_to_stack(&mut sp, stack_bottom, 0);
    for ptr in envp_ptrs {
        push_usize_to_stack(&mut sp, stack_bottom, ptr.value());
    }

    // Push argument pointers (`const char **argv`).
    push_usize_to_stack(&mut sp, stack_bottom, 0);
    for ptr in argv_ptrs {
        push_usize_to_stack(&mut sp, stack_bottom, ptr.value());
    }

    // Push argc.
    push_usize_to_stack(&mut sp, stack_bottom, argv.len());

    Ok(kernel_sp_to_user_sp(sp)?)
}

use kerla_runtime::{address::UserVAddr, arch::PAGE_SIZE};
use kerla_utils::alignment::is_aligned;

use crate::{
    ctypes::*, fs::opened_file::Fd, mm::vm::VmAreaType, prelude::*, process::current_process,
    syscalls::SyscallHandler,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_mmap(
        &mut self,
        addr_hint: Option<UserVAddr>,
        len: c_size,
        _prot: MMapProt,
        flags: MMapFlags,
        fd: Fd,
        offset: c_off,
    ) -> Result<isize> {
        // TODO: Respect `prot`.

        if !is_aligned(len as usize, PAGE_SIZE) {
            return Err(Errno::EINVAL.into());
        }

        if !is_aligned(offset as usize, PAGE_SIZE) {
            return Err(Errno::EINVAL.into());
        }

        let area_type = if flags.contains(MMapFlags::MAP_ANONYMOUS) {
            VmAreaType::Anonymous
        } else {
            let file = current_process()
                .opened_files()
                .lock()
                .get(fd)?
                .as_file()?
                .clone();

            VmAreaType::File {
                file,
                offset: offset as usize,
                file_size: len as usize,
            }
        };

        // Determine the virtual address space to map.
        let current = current_process();
        let vm_ref = current.vm();
        let mut vm = vm_ref.as_ref().unwrap().lock();
        let mapped_uaddr = match addr_hint {
            Some(addr_hint) if vm.is_free_vaddr_range(addr_hint, len as usize) => addr_hint,
            Some(_) => {
                // [addr_hint, addr_hint + len) is already in use or invalid.
                if flags.contains(MMapFlags::MAP_FIXED) {
                    return Err(Errno::EINVAL.into());
                }

                vm.alloc_vaddr_range(len as usize)?
            }
            None => vm.alloc_vaddr_range(len as usize)?,
        };

        vm.add_vm_area(mapped_uaddr, len as usize, area_type)?;
        Ok(mapped_uaddr.value() as isize)
    }
}

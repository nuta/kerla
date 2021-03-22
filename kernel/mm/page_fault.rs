use penguin_utils::alignment::align_down;

use super::{page_allocator::alloc_pages, vm::VmAreaType};
use crate::{
    arch::{PageFaultReason, UserVAddr, VAddr, PAGE_SIZE},
    process::current_process,
};
use core::cmp::min;
use core::slice;

pub fn handle_page_fault(unaligned_vaddr: UserVAddr, reason: PageFaultReason) {
    let vaddr = UserVAddr::new(align_down(unaligned_vaddr.value(), PAGE_SIZE)).unwrap();
    let current = current_process();
    let mut vm = current.vm.as_ref().unwrap().lock();

    // Look for the associated vma area.
    let vma = match vm.vm_areas().iter().find(|vma| vma.contains(vaddr)) {
        Some(vma) => vma,
        None => {
            // FIXME: Kill the current process
            todo!();
        }
    };

    // Allocate and fill the page.
    let paddr = alloc_pages(1).expect("failed to allocate an anonymous page");
    match vma.area_type() {
        VmAreaType::Anonymous => unsafe {
            paddr.as_mut_ptr::<u8>().write_bytes(0, PAGE_SIZE);
        },
        VmAreaType::File {
            file,
            offset,
            file_size,
        } => {
            let buf = unsafe { slice::from_raw_parts_mut(paddr.as_mut_ptr(), PAGE_SIZE) };
            let offset_in_vma = vma.offset_in_vma(vaddr);
            let zeroed_start = if offset_in_vma < *file_size {
                let end = min(*file_size - offset_in_vma, PAGE_SIZE);
                file.read(offset + offset_in_vma, &mut buf[..end])
                    .expect("failed to read file");
                end
            } else {
                0
            };

            // If p_memsz > p_filesz, the area beyond the file data must be filled
            // with zeroes.
            unsafe {
                buf[zeroed_start..]
                    .as_mut_ptr()
                    .write_bytes(0, buf.len() - zeroed_start);
            }
        }
    }

    // Map the page in the page table.
    vm.page_table_mut().map_user_page(vaddr, paddr);
}

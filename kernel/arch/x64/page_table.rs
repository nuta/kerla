use super::{PAddr, UserVAddr, PAGE_SIZE};
use crate::mm::page_allocator::{alloc_pages, AllocPageFlags};
use bitflags::bitflags;
use core::{
    debug_assert,
    ptr::{self, NonNull},
};
use penguin_utils::alignment::is_aligned;

type PageTableEntry = u64;

bitflags! {
    struct PageAttrs: u64 {
        const PRESENT = 1 << 0;
        const WRITABLE = 1 << 1;
        const USER = 1 << 2;
    }
}

bitflags! {
    pub struct PageFaultReason: u32 {
        const PRESENT = 1 << 0;
        const CAUSED_BY_WRITE = 1 << 1;
        const CAUSED_BY_USER = 1 << 2;
        const RESERVED_WRITE = 1 << 3;
        const CAUSED_BY_INST_FETCH = 1 << 4;
    }
}

fn entry_paddr(entry: PageTableEntry) -> PAddr {
    PAddr::new((entry & 0x7ffffffffffff000) as usize)
}

fn nth_level_table_index(vaddr: UserVAddr, level: usize) -> isize {
    ((vaddr.value() >> ((((level) - 1) * 9) + 12)) & 0x1ff) as isize
}

fn traverse(
    pml4: PAddr,
    vaddr: UserVAddr,
    allocate: bool,
    attrs: PageAttrs,
) -> Option<NonNull<PageTableEntry>> {
    debug_assert!(is_aligned(vaddr.value(), PAGE_SIZE));
    let mut table = unsafe { pml4.as_mut_ptr::<PageTableEntry>() };
    for level in (2..=4).rev() {
        let index = nth_level_table_index(vaddr, level);
        let entry = unsafe { table.offset(index) };
        let mut table_paddr = entry_paddr(unsafe { *entry });
        if table_paddr.value() == 0 {
            // The page table is not yet allocated.
            if !allocate {
                return None;
            }

            let new_table =
                alloc_pages(1, AllocPageFlags::KERNEL).expect("failed to allocate page table");
            unsafe {
                new_table.as_mut_ptr::<u8>().write_bytes(0, PAGE_SIZE);
                *entry = new_table.value() as u64 | attrs.bits()
            };

            table_paddr = new_table;
        }

        unsafe { *entry = table_paddr.value() as u64 | attrs.bits() };
        table = unsafe { table_paddr.as_mut_ptr::<PageTableEntry>() };
    }

    unsafe {
        Some(NonNull::new_unchecked(
            table.offset(nth_level_table_index(vaddr, 1)),
        ))
    }
}

pub struct PageTable {
    pml4: PAddr,
}

impl PageTable {
    pub fn new() -> PageTable {
        extern "C" {
            static __kernel_pml4: u8;
        }

        let pml4 = alloc_pages(1, AllocPageFlags::KERNEL).expect("failed to allocate page table");

        // Map kernel pages.
        unsafe {
            pml4.as_mut_ptr::<u8>().write_bytes(0, PAGE_SIZE);
            ptr::copy_nonoverlapping(&__kernel_pml4 as *const u8, pml4.as_mut_ptr(), PAGE_SIZE);
        }

        // The kernel no longer access a virtual address around 0x0000_0000. Unmap
        // the area to catch bugs (especially NULL pointer dereferences in the
        // kernel).
        //
        // TODO: Is it able to unmap in boot.S before running bsp_early_init?
        unsafe {
            *pml4.as_mut_ptr::<PageTableEntry>().offset(0) = 0;
        }

        PageTable { pml4 }
    }

    pub fn switch(&self) {
        unsafe {
            x86::controlregs::cr3_write(self.pml4.value() as u64);
        }
    }

    pub fn map_user_page(&mut self, vaddr: UserVAddr, paddr: PAddr) {
        self.map_page(
            vaddr,
            paddr,
            PageAttrs::PRESENT | PageAttrs::USER | PageAttrs::WRITABLE,
        );
    }

    fn map_page(&mut self, vaddr: UserVAddr, paddr: PAddr, attrs: PageAttrs) {
        debug_assert!(is_aligned(vaddr.value(), PAGE_SIZE));
        let mut entry = traverse(self.pml4, vaddr, true, attrs).unwrap();
        unsafe {
            *entry.as_mut() = paddr.value() as u64 | attrs.bits();
        }
    }
}

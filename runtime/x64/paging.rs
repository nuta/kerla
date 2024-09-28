use super::PAGE_SIZE;
use crate::address::{PAddr, UserVAddr};
use crate::page_allocator::{alloc_pages, AllocPageFlags, PageAllocError};
use bitflags::bitflags;
use core::{
    debug_assert,
    ptr::{self, NonNull},
};
use kerla_utils::alignment::is_aligned;

const ENTRIES_PER_TABLE: isize = 512;
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

fn entry_flags(entry: PageTableEntry) -> PageTableEntry {
    entry & !0x7ffffffffffff000
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
    let mut table = pml4.as_mut_ptr::<PageTableEntry>();
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
        table = table_paddr.as_mut_ptr::<PageTableEntry>();
    }

    unsafe {
        Some(NonNull::new_unchecked(
            table.offset(nth_level_table_index(vaddr, 1)),
        ))
    }
}

/// Duplicates entires (and referenced memory pages if `level == 1`) in the
/// nth-level page table. Returns the newly created copy of the page table.
///
/// fork(2) uses this funciton to duplicate the memory space.
fn duplicate_table(original_table_paddr: PAddr, level: usize) -> Result<PAddr, PageAllocError> {
    let orig_table = original_table_paddr.as_ptr::<PageTableEntry>();
    let new_table_paddr = alloc_pages(1, AllocPageFlags::KERNEL)?;
    let new_table = new_table_paddr.as_mut_ptr::<PageTableEntry>();

    debug_assert!(level > 0);
    for i in 0..ENTRIES_PER_TABLE {
        let entry = unsafe { *orig_table.offset(i) };
        let paddr = entry_paddr(entry);

        // Check if we need to copy the entry.
        if paddr.is_null() {
            continue;
        }

        // Create a deep copy of the page table entry.
        let new_paddr = if level == 1 {
            // Copy a physical page referenced from the last-level page table.
            let new_paddr = alloc_pages(1, AllocPageFlags::KERNEL)?;
            unsafe {
                ptr::copy_nonoverlapping::<u8>(paddr.as_ptr(), new_paddr.as_mut_ptr(), PAGE_SIZE);
            }
            new_paddr
        } else {
            // Copy the page table (PML4, PDPT, ...).
            if level == 4 && i >= 0x80 {
                // Kernel page table entries are immutable. Copy them as they are.
                entry_paddr(entry)
            } else {
                // Create the deep copy of the referenced page table recursively...
                duplicate_table(paddr, level - 1)?
            }
        };

        // Fill the new table's entry.
        unsafe {
            *new_table.offset(i) = new_paddr.value() as u64 | entry_flags(entry);
        }
    }

    Ok(new_table_paddr)
}

fn allocate_pml4() -> Result<PAddr, PageAllocError> {
    extern "C" {
        static __kernel_pml4: u8;
    }

    let pml4 = alloc_pages(1, AllocPageFlags::KERNEL)?;

    // Map kernel pages.
    unsafe {
        let kernel_pml4 = PAddr::new(&__kernel_pml4 as *const u8 as usize).as_vaddr();
        pml4.as_mut_ptr::<u8>().write_bytes(0, PAGE_SIZE);
        ptr::copy_nonoverlapping::<u8>(kernel_pml4.as_ptr(), pml4.as_mut_ptr(), PAGE_SIZE);
    }

    // The kernel no longer access a virtual address around 0x0000_0000. Unmap
    // the area to catch bugs (especially NULL pointer dereferences in the
    // kernel).
    //
    // TODO: Is it able to unmap in boot.S before running bsp_early_init?
    unsafe {
        *pml4.as_mut_ptr::<PageTableEntry>().offset(0) = 0;
    }

    Ok(pml4)
}

pub struct PageTable {
    pml4: PAddr,
}

impl PageTable {
    pub fn new() -> Result<PageTable, PageAllocError> {
        let pml4 = allocate_pml4()?;
        Ok(PageTable { pml4 })
    }

    pub fn duplicate_from(original: &PageTable) -> Result<PageTable, PageAllocError> {
        // TODO: Implement copy-on-write.
        Ok(PageTable {
            pml4: duplicate_table(original.pml4, 4)?,
        })
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

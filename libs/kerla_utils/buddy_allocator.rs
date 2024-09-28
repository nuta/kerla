use crate::alignment::{align_up, is_aligned};
use crate::byte_size::ByteSize;
use core::cmp::min;
use core::mem::size_of;
use core::ptr::NonNull;
use core::slice;

const PAGE_SIZE: usize = 4096;
const BUDDY_ORDER_MAX: usize = 10;

#[inline(always)]
fn pow2(order: usize) -> usize {
    1 << order
}

/// A physical memory page.
pub struct Page {
    /// The reference counter. 0 if the page is free.
    ref_count: usize,
    /// The intrusive pointer to the next chunk in a free list.
    next: Option<NonNull<Page>>,
}

impl Page {
    pub fn is_free(&self) -> bool {
        self.ref_count == 0
    }
}

/// A Last-In-First-Out (LIFO) intrusive queue.
struct FreeList {
    head: Option<NonNull<Page>>,
}

impl FreeList {
    pub const fn new() -> FreeList {
        FreeList { head: None }
    }

    /// Prepends a chunk.
    pub fn push(&mut self, mut new_tail: NonNull<Page>) {
        unsafe {
            new_tail.as_mut().next = self.head;
            self.head = Some(new_tail);
        }
    }

    /// Pops a chunk.
    pub fn pop(&mut self) -> Option<NonNull<Page>> {
        self.head.take().map(|head| {
            self.head = unsafe { head.as_ref().next };
            head
        })
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

/// A page frame allocator based on the buddy memory allocation algorithm.
///
/// In this allocator, memory pages are splitted into chunks: each chunk consists
/// of 2^n pages where n is called *order*.
///
/// # Memory Layout
///
/// ```text
/// +------------------------+  <-- base_paddr
/// |  array of Page struct  |
/// +------------------------+  <-- alloc_area_start
/// |        Page #0         |
/// |        Page #1         |
/// |        Page #2         |
/// |          ....          |
/// +------------------------+  <-- alloc_area_end
/// ```
pub struct BuddyAllocator {
    free_lists: [FreeList; BUDDY_ORDER_MAX],
    pages: NonNull<Page>,
    alloc_area_start: usize,
    alloc_area_end: usize,
}

impl BuddyAllocator {
    pub fn new(base: *mut u8, base_paddr: usize, len: usize) -> BuddyAllocator {
        debug_assert!(
            is_aligned(base_paddr, PAGE_SIZE),
            "base_paddr must be aligned to the page size"
        );
        debug_assert!(
            is_aligned(len, PAGE_SIZE),
            "len must be aligned to the page size"
        );

        const FREE_LIST: FreeList = FreeList::new();
        let mut free_lists = [FREE_LIST; BUDDY_ORDER_MAX];

        let total_num_pages = len / PAGE_SIZE;
        let pages_array_len = align_up(size_of::<Page>() * total_num_pages, PAGE_SIZE);
        let num_pages = total_num_pages - (pages_array_len / PAGE_SIZE);
        let pages = NonNull::new(base as *mut Page).unwrap();
        let alloc_area_start = base_paddr + pages_array_len;

        // Initialize the pages array.
        for i in 0..num_pages {
            let page = unsafe { &mut *pages.as_ptr().add(i) };
            page.ref_count = 0;
        }

        // Split into free lists.
        let mut i = 0;
        for order in (0..BUDDY_ORDER_MAX).rev() {
            while num_pages - i >= pow2(order) {
                let chunk = unsafe { NonNull::new_unchecked(pages.as_ptr().add(i)) };
                free_lists[order].push(chunk);
                i += pow2(order);
            }
        }

        BuddyAllocator {
            free_lists,
            pages,
            alloc_area_start,
            alloc_area_end: base_paddr + len,
        }
    }

    pub fn alloc_pages(&mut self, order: usize) -> Option<usize> {
        debug_assert!(order < BUDDY_ORDER_MAX);

        if self.free_lists[order].is_empty() {
            // The best-fit order's free list is empty. We need allocate and split
            // pages from higher orders to refill the list.
            self.refill_order(order);
        }

        // Try allocating from the best-fit order. It should have at least
        // two free areas or the kernel runs out of memory (the case `None`
        // is returned).
        self.free_lists[order].pop().map(|mut first_page| {
            // Increment reference counters of allocated pages.
            let base = self.page_to_paddr(first_page);
            for i in 0..pow2(order) {
                let mut page = self.paddr_to_page_mut(base + i * PAGE_SIZE).unwrap();
                debug_assert!(page.ref_count == 0);
                page.ref_count += 1;
            }

            base
        })
    }

    pub fn free_pages(&mut self, paddr: usize, order: usize) {
        debug_assert!(order < BUDDY_ORDER_MAX);

        // Decrement reference counters.
        for i in 0..pow2(order) {
            let page = self.paddr_to_page_mut(paddr + i * PAGE_SIZE).unwrap();
            debug_assert!(
                page.ref_count > 0,
                "page ref count is already zero -- perhaps a double free?"
            );
            page.ref_count -= 1;
        }

        // Try merging chunks into larger orders.
        let mut chunk_paddr = paddr;
        let mut chunk_order = order;
        'outer: for higher_order in (order + 1)..BUDDY_ORDER_MAX {
            let num_pages = pow2(chunk_order);
            let buddy_paddr = paddr ^ (num_pages * PAGE_SIZE);
            let larger_chunk_paddr = min(paddr, buddy_paddr);
            let larger_chunk_paddr_end = larger_chunk_paddr + pow2(higher_order) * PAGE_SIZE;

            // Check if the larger chunk is in the allocation area.
            if !self.is_paddr_in_allocation_area(larger_chunk_paddr) {
                break 'outer;
            }

            if !self.is_paddr_in_allocation_area(larger_chunk_paddr_end) {
                break 'outer;
            }

            // Check if all pages in the chunk are free.
            for i in 0..pow2(higher_order) {
                let page = self
                    .paddr_to_page_mut(larger_chunk_paddr + i * PAGE_SIZE)
                    .unwrap();
                if !page.is_free() {
                    break 'outer;
                }
            }

            // It seems the larger chunk can be merged.
            chunk_paddr = larger_chunk_paddr;
            chunk_order = higher_order;
        }

        self.add_chunk(chunk_order, chunk_paddr);
    }

    pub fn is_paddr_in_allocation_area(&self, paddr: usize) -> bool {
        self.alloc_area_start <= paddr && paddr < self.alloc_area_end
    }

    fn refill_order(&mut self, order: usize) {
        // Look for the lowest order containing at least one free entry.
        let available_order =
            ((order + 1)..BUDDY_ORDER_MAX).find(|order| !self.free_lists[*order].is_empty());

        if let Some(available_order) = available_order {
            for order in ((order + 1)..=available_order).rev() {
                // Split an entry into two entries with the lower order.
                let page = self.free_lists[order].pop().unwrap();
                let base = self.page_to_paddr(page);
                self.add_chunk(order - 1, base);
                self.add_chunk(order - 1, base + pow2(order - 1) * PAGE_SIZE);
            }
        }
    }

    fn add_chunk(&mut self, order: usize, paddr: usize) {
        self.free_lists[order].push(self.paddr_to_page(paddr).unwrap());
    }

    fn page_to_paddr(&self, page: NonNull<Page>) -> usize {
        let page_vaddr = page.as_ptr() as usize;
        let base_vaddr = self.pages.as_ptr() as usize;
        let pfn_offset = (page_vaddr - base_vaddr) / size_of::<Page>();
        self.alloc_area_start + pfn_offset * PAGE_SIZE
    }

    fn paddr_to_page(&self, paddr: usize) -> Option<NonNull<Page>> {
        if self.alloc_area_start <= paddr && paddr < self.alloc_area_end {
            let pfn_offset = (paddr - self.alloc_area_start) / PAGE_SIZE;
            Some(unsafe { NonNull::new_unchecked(self.pages.as_ptr().add(pfn_offset)) })
        } else {
            None
        }
    }

    fn paddr_to_page_mut(&mut self, paddr: usize) -> Option<&mut Page> {
        self.paddr_to_page(paddr)
            .map(|page| unsafe { &mut *page.as_ptr() })
    }
}

#[cfg(all(test, not(feature = "no_std")))]
mod tests {
    use super::*;
    use std::panic::catch_unwind;

    #[test]
    fn allocate_and_deallocate() {
        let len = 6 * PAGE_SIZE;
        let base_paddr = 0xccc0_0000;
        let mut base = vec![0u8; len];
        let mut allocator = BuddyAllocator::new(base.as_mut_slice().as_mut_ptr(), base_paddr, len);
        // 12345
        // .....
        let chunk_5 = allocator.alloc_pages(0);
        // 12345
        // ....X
        let chunk_3 = allocator.alloc_pages(1);
        // 12345
        // ..XXX
        let chunk_2 = allocator.alloc_pages(0);
        // 12345
        // .XXXX

        assert_eq!(chunk_5, Some(base_paddr + 5 * PAGE_SIZE));
        assert_eq!(chunk_3, Some(base_paddr + 3 * PAGE_SIZE));
        assert_eq!(chunk_2, Some(base_paddr + 2 * PAGE_SIZE));
        assert_eq!(allocator.alloc_pages(1), None);

        allocator.free_pages(chunk_5.unwrap(), 0);
        // 12345
        // .XXX.
        assert_eq!(allocator.alloc_pages(0), chunk_5);
        // 12345
        // .XXXX

        allocator.free_pages(chunk_5.unwrap(), 0);
        allocator.free_pages(chunk_3.unwrap(), 0);
        allocator.free_pages(chunk_2.unwrap(), 0);
        // 12345
        // .....
        assert_eq!(allocator.alloc_pages(1), Some(base_paddr + 2 * PAGE_SIZE));
        // 12345
        // XXX..
    }

    #[test]
    fn panics_on_double_free() {
        let len = 2 * PAGE_SIZE;
        let base_paddr = 0xccc0_0000;
        let mut base = vec![0u8; len];
        let mut allocator = BuddyAllocator::new(base.as_mut_slice().as_mut_ptr(), base_paddr, len);
        let allocated_paddr = allocator.alloc_pages(0);
        assert!(allocated_paddr.is_some());

        allocator.free_pages(allocated_paddr.unwrap(), 0);
        assert!(catch_unwind(move || allocator.free_pages(allocated_paddr.unwrap(), 0)).is_err());
    }
}

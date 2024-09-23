use core::ops::Deref;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{address::PAddr, arch::PAGE_SIZE, bootinfo::RamArea, spinlock::SpinLock};
use arrayvec::ArrayVec;
use bitflags::bitflags;
use kerla_utils::alignment::is_aligned;
use kerla_utils::byte_size::ByteSize;

use kerla_utils::bitmap_allocator::BitMapAllocator as Allocator;

// TODO: Fix bugs in use the buddy allocator.
// use kerla_utils::buddy_allocator::BuddyAllocator as Allocator;

// Comment out the following line to use BumpAllocator.
// use kerla_utils::bump_allocator::BumpAllocator as Allocator;

static ZONES: SpinLock<ArrayVec<Allocator, 8>> = SpinLock::new(ArrayVec::new_const());
static NUM_FREE_PAGES: AtomicUsize = AtomicUsize::new(0);
static NUM_TOTAL_PAGES: AtomicUsize = AtomicUsize::new(0);

fn num_pages_to_order(num_pages: usize) -> usize {
    // TODO: Use log2 instead
    for order in 0..16 {
        if num_pages > 1 << order {
            continue;
        }

        return order;
    }

    unreachable!();
}

#[derive(Debug)]
pub struct Stats {
    pub num_free_pages: usize,
    pub num_total_pages: usize,
}

pub fn read_allocator_stats() -> Stats {
    Stats {
        num_free_pages: NUM_FREE_PAGES.load(Ordering::SeqCst),
        num_total_pages: NUM_TOTAL_PAGES.load(Ordering::SeqCst),
    }
}

bitflags! {
    pub struct AllocPageFlags: u32 {
        // TODO: Currently both of them are unused in the allocator.

        /// Allocate pages for the kernel purpose.
        const KERNEL = 1 << 0;
        /// Allocate pages for the user.
        const USER = 1 << 1;
        /// If it's not set, allocated pages will be filled with zeroes.
        const DIRTY_OK = 1 << 2;
    }
}

#[derive(Debug)]
pub struct PageAllocError;

pub struct OwnedPages {
    paddr: PAddr,
    num_pages: usize,
}

impl OwnedPages {
    fn new(paddr: PAddr, num_pages: usize) -> OwnedPages {
        OwnedPages { paddr, num_pages }
    }
}

impl Deref for OwnedPages {
    type Target = PAddr;

    fn deref(&self) -> &Self::Target {
        &self.paddr
    }
}

impl Drop for OwnedPages {
    fn drop(&mut self) {
        free_pages(self.paddr, self.num_pages);
    }
}

// TODO: Use alloc_page
pub fn alloc_pages(num_pages: usize, flags: AllocPageFlags) -> Result<PAddr, PageAllocError> {
    let order = num_pages_to_order(num_pages);
    let mut zones = ZONES.lock();
    for zone in zones.iter_mut() {
        if let Some(paddr) = zone.alloc_pages(order).map(PAddr::new) {
            if !flags.contains(AllocPageFlags::DIRTY_OK) {
                unsafe {
                    paddr
                        .as_mut_ptr::<u8>()
                        .write_bytes(0, num_pages * PAGE_SIZE);
                }
            }

            NUM_FREE_PAGES.fetch_sub(num_pages, Ordering::SeqCst);
            return Ok(paddr);
        }
    }

    Err(PageAllocError)
}

pub fn alloc_pages_owned(
    num_pages: usize,
    flags: AllocPageFlags,
) -> Result<OwnedPages, PageAllocError> {
    let order = num_pages_to_order(num_pages);
    let mut zones = ZONES.lock();
    for zone in zones.iter_mut() {
        if let Some(paddr) = zone.alloc_pages(order).map(PAddr::new) {
            if !flags.contains(AllocPageFlags::DIRTY_OK) {
                unsafe {
                    paddr
                        .as_mut_ptr::<u8>()
                        .write_bytes(0, num_pages * PAGE_SIZE);
                }
            }

            NUM_FREE_PAGES.fetch_sub(num_pages, Ordering::SeqCst);
            return Ok(OwnedPages::new(paddr, num_pages));
        }
    }

    Err(PageAllocError)
}

/// The caller must ensure that the pages are not already freed. Keep holding
/// `OwnedPages` to free the pages in RAII basis.
pub fn free_pages(paddr: PAddr, num_pages: usize) {
    if cfg!(debug_assertions) {
        // Poison the memory.
        unsafe {
            paddr
                .as_mut_ptr::<u8>()
                .write_bytes(0xa5, num_pages * PAGE_SIZE);
        }
    }

    let order = num_pages_to_order(num_pages);
    let mut zones = ZONES.lock();
    for zone in zones.iter_mut() {
        if zone.includes(paddr.value()) {
            zone.free_pages(paddr.value(), order);
            NUM_FREE_PAGES.fetch_add(num_pages, Ordering::SeqCst);
            return;
        }
    }
}

pub fn init(areas: &[RamArea]) {
    let mut zones = ZONES.lock();
    for area in areas {
        info!(
            "available RAM: base={:x}, size={}",
            area.base.value(),
            ByteSize::new(area.len)
        );

        debug_assert!(is_aligned(area.base.value(), PAGE_SIZE));
        let allocator =
            unsafe { Allocator::new(area.base.as_mut_ptr(), area.base.value(), area.len) };
        NUM_FREE_PAGES.fetch_add(allocator.num_total_pages(), Ordering::SeqCst);
        NUM_TOTAL_PAGES.fetch_add(allocator.num_total_pages(), Ordering::SeqCst);
        zones.push(allocator);
    }
}

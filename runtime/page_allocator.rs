use core::ops::Deref;

use crate::{address::PAddr, arch::PAGE_SIZE, bootinfo::RamArea, spinlock::SpinLock};
use arrayvec::ArrayVec;
use bitflags::bitflags;
use kerla_utils::byte_size::ByteSize;

use kerla_utils::bitmap_allocator::BitMapAllocator as Allocator;

// TODO: Fix bugs in use the buddy allocator.
// use kerla_utils::buddy_allocator::BuddyAllocator as Allocator;

// Comment out the following line to use BumpAllocator.
// use kerla_utils::bump_allocator::BumpAllocator as Allocator;

static ZONES: SpinLock<ArrayVec<Allocator, 8>> = SpinLock::new(ArrayVec::new_const());

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

bitflags! {
    pub struct AllocPageFlags: u32 {
        // TODO: Currently both of them are unused in the allocator.

        /// Allocate pages for the kernel purpose.
        const KERNEL = 0;
        /// Allocate pages for the user.
        const USER = 0;
        /// Fill allocated pages with zeroes.
        const ZEROED = 1 << 0;
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

    fn leak(self) -> PAddr {
        self.paddr
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

pub fn alloc_pages(num_pages: usize, flags: AllocPageFlags) -> Result<PAddr, PageAllocError> {
    let order = num_pages_to_order(num_pages);
    let mut zones = ZONES.lock();
    for zone in zones.iter_mut() {
        if let Some(paddr) = zone.alloc_pages(order).map(PAddr::new) {
            // if flags.contains(AllocPageFlags::ZEROED) {
            unsafe {
                paddr
                    .as_mut_ptr::<u8>()
                    .write_bytes(0, num_pages * PAGE_SIZE);
            }
            // }

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
            // if flags.contains(AllocPageFlags::ZEROED) {
            unsafe {
                paddr
                    .as_mut_ptr::<u8>()
                    .write_bytes(0, num_pages * PAGE_SIZE);
            }
            // }

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

        zones.push(Allocator::new(
            area.base.as_mut_ptr(),
            area.base.value(),
            area.len,
        ));
    }
}

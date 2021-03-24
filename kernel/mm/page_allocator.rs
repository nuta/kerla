use crate::{
    arch::{PAddr, SpinLock},
    boot::RamArea,
};
use arrayvec::ArrayVec;
use bitflags::bitflags;
use penguin_utils::bump_allocator::BumpAllocator as Allocator;
use penguin_utils::byte_size::ByteSize;
// TODO:
// use penguin_utils::buddy_allocator::BuddyAllocator as Allocator;

static ZONES: SpinLock<ArrayVec<[Allocator; 8]>> = SpinLock::new(ArrayVec::new());

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
    }
}

pub fn alloc_pages(num_pages: usize, flags: AllocPageFlags) -> Option<PAddr> {
    let order = num_pages_to_order(num_pages);
    let mut zones = ZONES.lock();
    for i in 0..zones.len() {
        if let Some(paddr) = zones[i].alloc_pages(order) {
            return Some(PAddr::new(paddr));
        }
    }

    None
}

pub fn init(areas: &[RamArea]) {
    let mut zones = ZONES.lock();
    for area in areas {
        println!(
            "available RAM: base={:x}, size={}",
            area.base.value(),
            ByteSize::new(area.len)
        );

        zones.push(Allocator::new(
            unsafe { area.base.as_mut_ptr() },
            area.base.value(),
            area.len,
        ));
    }
}

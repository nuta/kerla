// use core::sync::atomic::{AtomicBool, Ordering};

use buddy_system_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap<32 /* order */> = LockedHeap::empty();
// static KERNEL_HEAP_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn is_kernel_heap_enabled() -> bool {
    todo!();
    // KERNEL_HEAP_ENABLED.load(Ordering::Acquire)
}

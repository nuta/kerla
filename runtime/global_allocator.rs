use core::alloc::Layout;
use core::sync::atomic::{AtomicBool, Ordering};

use buddy_system_allocator::{Heap, LockedHeapWithRescue};
use kerla_utils::alignment::align_up;

use crate::arch::PAGE_SIZE;
use crate::page_allocator::{alloc_pages, AllocPageFlags};

const ORDER: usize = 32;
const KERNEL_HEAP_CHUNK_SIZE: usize = 1024 * 1024; // 1MiB

#[global_allocator]
static ALLOCATOR: LockedHeapWithRescue<ORDER> = LockedHeapWithRescue::new(expand_kernel_heap);
static KERNEL_HEAP_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn is_kernel_heap_enabled() -> bool {
    KERNEL_HEAP_ENABLED.load(Ordering::Acquire)
}

fn expand_kernel_heap(heap: &mut Heap<ORDER>, layout: &Layout) {
    if layout.size() > KERNEL_HEAP_CHUNK_SIZE {
        panic!(
            "tried to allocate too large object in the kernel heap (requested {} bytes)",
            layout.size()
        );
    }

    let num_pages = align_up(KERNEL_HEAP_CHUNK_SIZE, PAGE_SIZE) / PAGE_SIZE;
    let start = alloc_pages(num_pages, AllocPageFlags::KERNEL)
        .expect("run out of memory: failed to expand the kernel heap")
        .as_vaddr()
        .value();
    let end = start + KERNEL_HEAP_CHUNK_SIZE;

    unsafe {
        heap.add_to_heap(start, end);
    }

    KERNEL_HEAP_ENABLED.store(true, Ordering::Release)
}

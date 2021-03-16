use crate::boot::RamArea;
use crate::utils::byte_size::ByteSize;
use buddy_system_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("alloc error: layout={:?}", layout);
}

pub fn init(areas: &[RamArea]) {
    for area in areas {
        println!(
            "available RAM: base={:x}, size={}",
            area.base.value(),
            ByteSize::new(area.len)
        );
        unsafe {
            ALLOCATOR
                .lock()
                .add_to_heap(area.base.value(), area.base.add(area.len).value());
        }
    }
}

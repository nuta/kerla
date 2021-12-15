const PAGE_SIZE: usize = 4096;

pub struct BumpAllocator {
    base: usize,
    current: usize,
    end: usize,
}

impl BumpAllocator {
    /// # Safety
    ///
    /// The caller must ensure that the memory passed to this function is
    /// aligned to a page boundary.
    pub unsafe fn new(_base: *mut u8, base_paddr: usize, len: usize) -> BumpAllocator {
        BumpAllocator {
            base: base_paddr,
            current: base_paddr,
            end: base_paddr + len,
        }
    }

    pub fn includes(&mut self, ptr: usize) -> bool {
        self.base <= ptr && ptr < self.end
    }

    pub fn alloc_pages(&mut self, order: usize) -> Option<usize> {
        let len = PAGE_SIZE * (1 << order);
        if self.current + len >= self.end {
            return None;
        }

        let ptr = self.current;
        self.current += len;
        Some(ptr)
    }

    pub fn free_pages(&mut self, _ptr: usize, _order: usize) {
        // Not supported.
    }
}

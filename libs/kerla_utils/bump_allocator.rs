const PAGE_SIZE: usize = 4096;

pub struct BumpAllocator {
    current: usize,
    end: usize,
}

impl BumpAllocator {
    pub fn new(_base: *mut u8, base_paddr: usize, len: usize) -> BumpAllocator {
        BumpAllocator {
            current: base_paddr,
            end: base_paddr + len,
        }
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
}

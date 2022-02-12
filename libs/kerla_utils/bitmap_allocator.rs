use bitvec::{index::BitMask, prelude::*};

use crate::alignment::align_up;

const PAGE_SIZE: usize = 4096;

pub struct BitMapAllocator {
    bitmap: spin::Mutex<&'static mut BitSlice<u8, LocalBits>>,
    base: usize,
    end: usize,
}

impl BitMapAllocator {
    /// # Safety
    ///
    /// The caller must ensure that the memory passed to this function is
    /// aligned to a page boundary.
    pub unsafe fn new(base: *mut u8, base_paddr: usize, len: usize) -> BitMapAllocator {
        let num_pages = align_up(len, PAGE_SIZE) / PAGE_SIZE;
        let bitmap_reserved_len = align_up(num_pages / 8, PAGE_SIZE);
        let bitmap_actual_len = (num_pages / 8) - (bitmap_reserved_len / PAGE_SIZE);
        let bitmap =
            BitSlice::from_slice_mut(core::slice::from_raw_parts_mut(base, bitmap_actual_len));

        debug_assert!(bitmap_reserved_len >= bitmap_actual_len);
        bitmap.fill(false);

        BitMapAllocator {
            bitmap: spin::Mutex::new(bitmap),
            base: base_paddr + bitmap_reserved_len,
            end: base_paddr + len - bitmap_reserved_len,
        }
    }

    pub fn num_total_pages(&self) -> usize {
        (self.end - self.base) / PAGE_SIZE
    }

    pub fn includes(&mut self, ptr: usize) -> bool {
        self.base <= ptr && ptr < self.end
    }

    pub fn alloc_pages(&mut self, order: usize) -> Option<usize> {
        let num_pages = 1 << order;
        let mut bitmap = self.bitmap.lock();
        let mut off = 0;
        while let Some(first_zero) = bitmap[off..].first_zero() {
            let start = off + first_zero;
            let end = off + first_zero + num_pages;
            if end > bitmap.len() {
                break;
            }

            if bitmap[start..end].not_any() {
                bitmap[start..end].fill(true);
                return Some(self.base + start * PAGE_SIZE);
            }

            off += first_zero + 1;
        }

        None
    }

    pub fn free_pages(&mut self, ptr: usize, order: usize) {
        let num_pages = 1 << order;
        let off = (ptr - self.base) / PAGE_SIZE;

        let mut bitmap = self.bitmap.lock();

        debug_assert!(bitmap[off..(off + num_pages)].all(), "double free");
        bitmap[off..(off + num_pages)].fill(false);
    }
}

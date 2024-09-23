use bitvec::prelude::*;
use core::mem::size_of;

pub struct IdTable<const BIT_LENGTH: usize>(BitArray<[usize; BIT_LENGTH], LocalBits>); // const expr arithmetic unstable

impl<const BIT_LENGTH: usize> IdTable<BIT_LENGTH> {
    pub const fn new() -> IdTable<BIT_LENGTH> {
        IdTable(BitArray::ZERO)
    }

    pub fn alloc(&mut self) -> Option<usize> {
        self.0.first_zero().map(|id| {
            self.0.set(id, true);
            id
        })
    }

    pub fn free(&mut self, id: usize) {
        debug_assert_eq!(self.0.get(id).as_deref(), Some(&true));
        self.0.set(id, false);
    }
}

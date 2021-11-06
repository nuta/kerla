use crate::bitmap::BitMap;

pub struct IdTable<const SZ: usize>(BitMap<SZ>);

impl<const SZ: usize> IdTable<SZ> {
    pub const fn new() -> IdTable<SZ> {
        IdTable(BitMap::zeroed())
    }

    pub fn alloc(&mut self) -> Option<usize> {
        self.0.first_zero().map(|id| {
            self.0.set(id);
            id
        })
    }

    pub fn free(&mut self, id: usize) {
        debug_assert_eq!(self.0.get(id), Some(true));
        self.0.unset(id);
    }
}

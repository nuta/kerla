use core::slice::memchr::memchr;

pub struct BitMap<const SZ: usize>([u8; SZ]);

impl<const SZ: usize> BitMap<SZ> {
    pub const fn zeroed() -> BitMap<SZ> {
        BitMap([0; SZ])
    }

    #[cfg(test)]
    pub const fn from_array(array: [u8; SZ]) -> BitMap<SZ> {
        BitMap(array)
    }

    #[cfg(test)]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    #[inline]
    pub fn bits(&self) -> usize {
        SZ * 8
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.bits() {
            return None;
        }

        Some(self.0[index / 8] & (1 << (index % 8)) != 0)
    }

    #[inline(always)]
    pub fn set(&mut self, index: usize) {
        self.0[index / 8] |= 1 << (index % 8);
    }

    #[inline(always)]
    pub fn unset(&mut self, index: usize) {
        self.0[index / 8] &= !(1 << (index % 8));
    }

    #[inline]
    pub fn first_zero(&self) -> Option<usize> {
        for (byte_index, byte) in self.0.iter().enumerate() {
            if *byte != 0xff {
                return Some(byte_index * 8 + (byte.trailing_ones() as usize));
            }
        }

        None
    }
}

#[cfg(all(test, not(feature = "no_std")))]
mod tests {
    use super::*;

    #[test]
    fn test_bit_map() {
        let mut bitmap = BitMap::from_array([0xff, 0xff, 0xff]);
        assert_eq!(bitmap.first_zero(), None);
        bitmap.unset(13);
        assert_eq!(bitmap.first_zero(), Some(13));

        let mut bitmap = BitMap::from_array([0b1111_1111, 0b1111_0001]);
        assert_eq!(bitmap.get(11), Some(false));
        bitmap.set(11);
        assert_eq!(bitmap.get(11), Some(true));
        assert_eq!(bitmap.as_slice(), &[0b1111_1111, 0b1111_1001]);
        assert_eq!(bitmap.first_zero(), Some(9));
    }
}

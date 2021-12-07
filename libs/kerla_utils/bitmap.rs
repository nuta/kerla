use core::slice::memchr::memchr;

#[derive(Clone)]
pub struct BitMap<const SZ: usize>([u8; SZ]);

impl<const SZ: usize> BitMap<SZ> {
    pub const fn zeroed() -> BitMap<SZ> {
        BitMap([0; SZ])
    }

    #[cfg(test)]
    pub const fn from_array(array: [u8; SZ]) -> BitMap<SZ> {
        BitMap(array)
    }

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

    pub fn assign(&mut self, rhs: [u8; SZ]) {
        self.0 = rhs;
    }

    /// This method will panic if SZ != rhs.len()
    pub fn assign_or(&mut self, rhs: [u8; SZ]) {
        for (i, byte) in self.0.iter_mut().enumerate() {
            *byte |= rhs[i];
        }
    }

    /// This method will panic if SZ != rhs.len()
    pub fn assign_and_not(&mut self, rhs: [u8; SZ]) {
        for (i, byte) in self.0.iter_mut().enumerate() {
            *byte &= !rhs[i];
        }
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

    #[test]
    fn or() {
        let mut bitmap = BitMap::from_array([0b0100_0010, 0b1000_0001]);
        bitmap.assign_or([0b0010_0100, 0b1010_0110]);
        assert_eq!(bitmap.as_slice(), &[0b0110_0110, 0b1010_0111]);
    }

    #[test]
    fn assign_and_not() {
        let mut bitmap = BitMap::from_array([0b0100_0010, 0b1000_0001]);
        bitmap.assign_and_not([0b0110_0100, 0b1010_0110]);
        assert_eq!(bitmap.as_slice(), &[0b0000_0010, 0b0000_0001]);
    }
}

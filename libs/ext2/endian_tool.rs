pub type U32le = u32;
pub type U16le = u16;
pub mod read_func {
    use core::mem::size_of;
    use super::{U32le, U16le};

    pub fn read_u8<'a>(data: &'a mut &[u8]) -> (&'a [u8], u8) {
        let (int_bytes, rest) = data.split_at(size_of::<u8>());
        let num = u8::from_le_bytes(int_bytes.try_into().unwrap());
        (rest, num)
    }

    pub fn read_u816<'a>(data: &'a mut &[u8]) -> (&'a [u8], [u8; 16]) {
        let (int_bytes, rest) = data.split_at(size_of::<u8>() * 16);
        let result: [u8; 16] = <[u8; 16]>::try_from(int_bytes).unwrap();
        (rest, result)
    }

    /// read U32le by binary.
    /// return new vec<u8> and num
    pub fn read_u32le<'a>(data: &'a mut &[u8]) -> (&'a [u8], U32le){
        let (int_bytes, rest) = data.split_at(size_of::<U32le>());
        let num = U32le::from_le_bytes(int_bytes.try_into().unwrap());
        (rest, num)
    }

    pub fn read_u32<'a>(data: &'a mut &[u8]) -> (&'a [u8], u32){
        let (int_bytes, rest) = data.split_at(size_of::<u32>());
        let num = u32::from_le_bytes(int_bytes.try_into().unwrap());
        (rest, num)
    }

    pub fn read_u324<'a>(data: &'a mut &[u8]) -> (&'a [u8], [u32; 4]){
        let (int_bytes, rest) = data.split_at(size_of::<u32>() * 4);
        let size = size_of::<u32>();
        let mut result = [0u32; 4];
        for (index, _) in (0..4).enumerate() {
            let start = index * size;
            let num = u32::from_ne_bytes(int_bytes[start..start + 4].try_into().unwrap());
            result[index] = num;
        }
        (rest, result)
    }

    pub fn read_u16le<'a>(data: &'a mut &[u8]) -> (&'a [u8], U16le) {
        let (int_bytes, rest) = data.split_at(size_of::<U16le>());
        let num = U16le::from_le_bytes(int_bytes.try_into().unwrap());
        (rest, num)
    }

    pub fn read_u16<'a>(data: &'a mut &[u8]) -> (&'a [u8], u16) {
        let (int_bytes, rest) = data.split_at(size_of::<u16>());
        let num = u16::from_ne_bytes(int_bytes.try_into().unwrap());
        (rest, num)
    }

    pub fn read_char16<'a>(data: &'a mut &[u8]) -> (&'a [u8], [char; 16]) {
        let (bytes, rest) = data.split_at(16);
        let mut chars = [' '; 16];
        let mut index = 0;
        for byte in bytes {
            chars[index] = char::from(*byte);
        }
        (rest, chars)
    }

    pub fn read_char64<'a>(data: &'a mut &[u8]) -> (&'a [u8], [char; 64]) {
        let (bytes, rest) = data.split_at(64);
        let mut chars = [' '; 64];
        let mut index = 0;
        for byte in bytes {
            chars[index] = char::from(*byte);
        }
        (rest, chars)
    }
}
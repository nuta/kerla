use core::convert::TryInto;

fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}

fn align_up(value: usize, align: usize) -> usize {
    align_down(value + align - 1, align)
}

#[derive(Debug, PartialEq)]
pub enum BytesParserError {
    TooShort,
}

pub struct BytesParser<'a> {
    buffer: &'a [u8],
    current: usize,
}

impl<'a> BytesParser<'a> {
    pub fn new(buffer: &'a [u8]) -> BytesParser<'a> {
        BytesParser { buffer, current: 0 }
    }

    pub fn remaining(&self) -> &[u8] {
        &self.buffer[self.current..]
    }

    pub fn remaining_len(&self) -> usize {
        self.buffer.len() - self.current
    }

    pub fn skip(&mut self, len: usize) -> Result<(), BytesParserError> {
        if self.current + len > self.buffer.len() {
            return Err(BytesParserError::TooShort);
        }

        self.current += len;
        Ok(())
    }

    pub fn skip_until_alignment(&mut self, align: usize) -> Result<(), BytesParserError> {
        let next = align_up(self.current, align);
        if next > self.buffer.len() {
            return Err(BytesParserError::TooShort);
        }

        self.current = next;
        Ok(())
    }

    pub fn consume_bytes(&mut self, len: usize) -> Result<&'a [u8], BytesParserError> {
        if self.current + len > self.buffer.len() {
            return Err(BytesParserError::TooShort);
        }

        self.current += len;
        Ok(&self.buffer[self.current - len..self.current])
    }

    pub fn consume_le_u16(&mut self) -> Result<u16, BytesParserError> {
        if self.remaining_len() < 2 {
            return Err(BytesParserError::TooShort);
        }

        let value = u16::from_le_bytes(
            self.buffer[self.current..self.current + 2]
                .try_into()
                .unwrap(),
        );
        self.current += 2;
        Ok(value)
    }

    pub fn consume_le_u32(&mut self) -> Result<u32, BytesParserError> {
        if self.remaining_len() < 4 {
            return Err(BytesParserError::TooShort);
        }

        let value = u32::from_le_bytes(
            self.buffer[self.current..self.current + 4]
                .try_into()
                .unwrap(),
        );
        self.current += 4;
        Ok(value)
    }

    pub fn consume_le_u64(&mut self) -> Result<u64, BytesParserError> {
        if self.remaining_len() < 8 {
            return Err(BytesParserError::TooShort);
        }

        let value = u64::from_le_bytes(
            self.buffer[self.current..self.current + 8]
                .try_into()
                .unwrap(),
        );
        self.current += 8;
        Ok(value)
    }

    pub fn consume_le_i32(&mut self) -> Result<i32, BytesParserError> {
        if self.remaining_len() < 4 {
            return Err(BytesParserError::TooShort);
        }

        let value = i32::from_le_bytes(
            self.buffer[self.current..self.current + 4]
                .try_into()
                .unwrap(),
        );
        self.current += 4;
        Ok(value)
    }
}

use core::fmt;

#[repr(transparent)]
pub struct ByteSize(usize);

impl ByteSize {
    pub const fn new(value: usize) -> ByteSize {
        ByteSize(value)
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let units = &["B", "KiB", "MiB", "GiB", "TiB"];
        let mut value = self.0;
        let mut i = 0;
        let mut unit = units[0];
        while value >= 1024 && i + 1 < units.len() {
            value /= 1024;
            unit = units[i + 1];
            i += 1;
        }

        write!(f, "{}{}", value, unit)
    }
}

pub trait NetworkEndianExt {
    fn from_network_endian(self) -> Self;
}

impl NetworkEndianExt for u16 {
    fn from_network_endian(self) -> Self {
        if cfg!(target_endian = "big") {
            self
        } else {
            ((self & 0xff00) >> 8) | ((self & 0x00ff) << 8)
        }
    }
}

impl NetworkEndianExt for u32 {
    fn from_network_endian(self) -> Self {
        if cfg!(target_endian = "big") {
            self
        } else {
            ((self & 0xff000000) >> 24)
                | ((self & 0x00ff0000) >> 8)
                | ((self & 0x0000ff00) << 8)
                | ((self & 0x000000ff) << 24)
        }
    }
}

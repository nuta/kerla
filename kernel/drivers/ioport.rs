use x86::io::{inb, inl, inw, outb, outl, outw};

#[derive(Debug, Copy, Clone)]
pub struct IoPort {
    base: u16,
}

impl IoPort {
    pub const fn new(base: u16) -> IoPort {
        IoPort { base }
    }

    pub fn read8(&self, reg: u16) -> u8 {
        unsafe { inb(self.base + reg) }
    }

    pub fn read16(&self, reg: u16) -> u16 {
        unsafe { inw(self.base + reg) }
    }

    pub fn read32(&self, reg: u16) -> u32 {
        unsafe { inl(self.base + reg) }
    }

    pub fn write8(&self, reg: u16, value: u8) {
        unsafe { outb(self.base + reg, value) }
    }

    pub fn write16(&self, reg: u16, value: u16) {
        unsafe { outw(self.base + reg, value) }
    }

    pub fn write32(&self, reg: u16, value: u32) {
        unsafe { outl(self.base + reg, value) }
    }
}

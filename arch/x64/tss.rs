use super::gdt::TSS_SEG;
use x86::{segmentation::SegmentSelector, task::load_tr};

pub const IST_RSP0: u8 = 0;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct Tss {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    iomap_offset: u16,
    iomap: [u8; 8191],
    iomap_last_byte: u8,
}

impl Tss {
    pub fn set_rsp0(&mut self, rsp0: u64) {
        self.rsp0 = rsp0;
    }
}

cpu_local! {
    pub static ref TSS: Tss = Tss {
        reserved0: 0,
        rsp0: 0,
        rsp1: 0,
        rsp2: 0,
        reserved1: 0,
        ist: [0; 7],
        reserved2: 0,
        reserved3: 0,
        iomap_offset: 104, // offsetof(Tss, iomap)
        iomap: [0; 8191],
        // According to Intel SDM, all bits of the last byte must be set to 1.
        iomap_last_byte: 0xff,
    };
}

pub unsafe fn init() {
    load_tr(SegmentSelector::from_raw(TSS_SEG));
}

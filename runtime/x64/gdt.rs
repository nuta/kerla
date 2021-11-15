use super::tss::TSS;
use core::convert::TryInto;
use core::mem::size_of;
use x86::dtables::{lgdt, DescriptorTablePointer};

pub const KERNEL_CS: u16 = 8;
pub const USER_CS32: u16 = 24;
pub const USER_DS: u16 = 32; // Note: it's hard coded in kernel's usermode.S
pub const USER_CS64: u16 = 40; // Note: it's hard coded in kernel's usermode.S
pub const TSS_SEG: u16 = 48;
pub const USER_RPL: u16 = 3;

cpu_local! {
    static ref GDT: [u64; 8] = [
    0x0000000000000000, // null
    0x00af9a000000ffff, // kernel_cs
    0x00af92000000ffff, // kernel_ds
    0x0000000000000000, // user_cs32
    0x008ff2000000ffff, // user_ds
    0x00affa000000ffff, // user_cs64
    0,                  // tss_low
    0,                  // tss_high
];
}

pub unsafe fn init() {
    // Fill the TSS descriptor.
    let tss_addr = TSS.vaddr().value() as u64;
    let gdt = GDT.as_mut();
    gdt[(TSS_SEG as usize) / 8] = 0x0000890000000000
        | (size_of::<GDT>() as u64)
        | ((tss_addr & 0xffff) << 16)
        | (((tss_addr >> 16) & 0xff) << 32)
        | (((tss_addr >> 24) & 0xff) << 56);
    gdt[(TSS_SEG as usize) / 8 + 1] = tss_addr >> 32;

    let base = GDT.vaddr().as_ptr::<u64>();
    let limit = (GDT.get().len() * size_of::<u64>() - 1).try_into().unwrap();
    lgdt(&DescriptorTablePointer { limit, base });
}

use super::gdt::KERNEL_CS;
use super::tss::IST_RSP0;
use core::mem::size_of;
use x86::dtables::{lidt, DescriptorTablePointer};

const HANDLER_SIZE: usize = 16;
const NUM_IDT_DESCS: usize = 256;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IdtEntry {
    offset1: u16,
    seg: u16,
    ist: u8,
    info: u8,
    offset2: u16,
    offset3: u32,
    reserved: u32,
}

// TODO: CPU-local
static mut IDT: [IdtEntry; NUM_IDT_DESCS] = [IdtEntry {
    offset1: 0,
    seg: 0,
    ist: 0,
    info: 0,
    offset2: 0,
    offset3: 0,
    reserved: 0,
}; NUM_IDT_DESCS];

extern "C" {
    static interrupt_handlers: [[u8; HANDLER_SIZE]; NUM_IDT_DESCS];
}

pub unsafe fn init() {
    for i in 0..NUM_IDT_DESCS {
        let handler = &interrupt_handlers[i] as *const _ as u64;
        IDT[i].offset1 = (handler & 0xffff) as u16;
        IDT[i].seg = KERNEL_CS;
        IDT[i].ist = IST_RSP0;
        IDT[i].info = 0x8e;
        IDT[i].offset2 = ((handler >> 16) & 0xffff) as u16;
        IDT[i].offset3 = ((handler >> 32) & 0xffffffff) as u32;
        IDT[i].reserved = 0;
    }

    let base = &IDT;
    let limit = (IDT.len() * size_of::<IdtEntry>() - 1) as u16;
    lidt(&DescriptorTablePointer { limit, base });
}

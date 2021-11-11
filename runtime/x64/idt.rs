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

cpu_local! {
    static ref IDT: [IdtEntry; NUM_IDT_DESCS] = [IdtEntry {
        offset1: 0,
        seg: 0,
        ist: 0,
        info: 0,
        offset2: 0,
        offset3: 0,
        reserved: 0,
    }; NUM_IDT_DESCS];
}

extern "C" {
    static interrupt_handlers: [[u8; HANDLER_SIZE]; NUM_IDT_DESCS];
}

pub unsafe fn init() {
    for i in 0..NUM_IDT_DESCS {
        let handler = &interrupt_handlers[i] as *const _ as u64;
        let idt = IDT.as_mut();
        idt[i].offset1 = (handler & 0xffff) as u16;
        idt[i].seg = KERNEL_CS;
        idt[i].ist = IST_RSP0;
        idt[i].info = 0x8e;
        idt[i].offset2 = ((handler >> 16) & 0xffff) as u16;
        idt[i].offset3 = ((handler >> 32) & 0xffffffff) as u32;
        idt[i].reserved = 0;
    }

    let base = IDT.vaddr().as_ptr::<IdtEntry>();
    let limit = (IDT.get().len() * size_of::<IdtEntry>() - 1) as u16;
    lidt(&DescriptorTablePointer { limit, base });
}

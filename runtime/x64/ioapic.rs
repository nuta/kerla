use crate::address::PAddr;
use crate::spinlock::SpinLock;
use core::ptr::{read_volatile, write_volatile};
use x86::io::outb;

/// The base index of interrupt vectors.
pub const VECTOR_IRQ_BASE: u8 = 32;

static IO_APIC: SpinLock<IoApic> = SpinLock::new(IoApic::new(PAddr::new(0xfec0_0000)));

#[repr(u32)]
enum IoApicReg {
    Ver = 0x0,
    RedirectTableBase = 0x10,
}

struct IoApic {
    base: PAddr,
}

impl IoApic {
    pub const fn new(base: PAddr) -> IoApic {
        IoApic { base }
    }

    pub unsafe fn read_ver(&self) -> u32 {
        self.read(IoApicReg::Ver as u32)
    }

    pub unsafe fn write_iored_tbl(&self, index: u32, value: u64) {
        let reg_index = (IoApicReg::RedirectTableBase as u32) + index * 2;
        self.write(reg_index, (value & 0xffff_ffff) as u32);
        self.write(reg_index + 1, ((value >> 32) & 0xffff_ffff) as u32);
    }

    /// The index register.
    #[inline(always)]
    pub unsafe fn ind_reg(&self) -> *mut u32 {
        self.base.as_mut_ptr()
    }

    /// The data register.
    #[inline(always)]
    pub unsafe fn dat_reg(&self) -> *mut u32 {
        self.base.add(0x10).as_mut_ptr()
    }

    #[inline(always)]
    unsafe fn read(&self, index: u32) -> u32 {
        write_volatile(self.ind_reg(), index);
        read_volatile(self.dat_reg())
    }

    #[inline(always)]
    unsafe fn write(&self, index: u32, value: u32) {
        write_volatile(self.ind_reg(), index);
        write_volatile(self.dat_reg(), value);
    }
}

pub fn enable_irq(irq: u8) {
    let ioapic = IO_APIC.lock();
    unsafe {
        let entry = (VECTOR_IRQ_BASE as u64) + (irq as u64);
        ioapic.write_iored_tbl(irq as u32, entry);
    }
}

pub unsafe fn init() {
    // symmetric I/O mode.
    // FIXME: Do we need this?
    outb(0x22, 0x70);
    outb(0x23, 0x01);

    // Mask (disable) all hardware interrupts for now.
    let ioapic = IO_APIC.lock();
    let n = ((ioapic.read_ver() >> 16) & 0xff) + 1;
    for i in 0..n {
        ioapic.write_iored_tbl(i, 1 << 16 /* masked */);
    }
}

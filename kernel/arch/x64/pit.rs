use super::ioapic::enable_irq;
use x86::io::outb;

const DIVISOR: u16 = (1193182u32 / 1000) as u16;

pub unsafe fn init() {
    trace!("enabling PIT (i8254) timer: divisor={}", DIVISOR);
    outb(0x43, 0x35);
    outb(0x40, (DIVISOR & 0xff) as u8);
    outb(0x40, (DIVISOR >> 8) as u8);

    // FIXME: Check "Legacy replacement" mapping
    enable_irq(0);
    enable_irq(2);
}

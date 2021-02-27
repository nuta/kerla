use crate::boot::boot_kernel;

global_asm!(include_str!("boot.S"));

const IOPORT_SERIAL: u16 = 0x3f8;
const DLL: u16 = 0;
const DLH: u16 = 1;
const IER: u16 = 1;
const FCR: u16 = 2;
const LCR: u16 = 3;
const LSR: u16 = 5;
const TX_READY: u8 = 0x20;

unsafe fn asm_out8(port: u16, value: u8) {
    asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
    );
}

unsafe fn asm_in8(port: u16) -> u8 {
    let mut value;
    asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
    );
    value
}

unsafe fn serial_write(ch: char) {
    while (asm_in8(IOPORT_SERIAL + LSR) & TX_READY) == 0 {}
    asm_out8(IOPORT_SERIAL, ch as u8);
}

pub fn printchar(ch: char) {
    unsafe {
        serial_write(ch);
        if ch == '\n' {
            serial_write('\r');
        }
    }
}

unsafe fn serial_init() {
    let divisor: u16 = 12; // 115200 / 9600 = 12
    asm_out8(IOPORT_SERIAL + IER, 0x00); // Disable interrupts.
    asm_out8(IOPORT_SERIAL + DLL, (divisor & 0xff) as u8);
    asm_out8(IOPORT_SERIAL + DLH, ((divisor >> 8) & 0xff) as u8);
    asm_out8(IOPORT_SERIAL + LCR, 0x03); // 8n1.
    asm_out8(IOPORT_SERIAL + FCR, 0x01); // Enable FIFO.
    asm_out8(IOPORT_SERIAL + IER, 0x01); // Enable interrupts.
}

#[no_mangle]
pub unsafe extern "C" fn init() -> ! {
    serial_init();
    printchar('\n');
    boot_kernel();
    loop {}
}

#[no_mangle]
pub extern "C" fn mpinit() -> ! {
    loop {}
}

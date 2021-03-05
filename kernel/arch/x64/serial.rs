use x86::io::{inb, outb};

const IOPORT_SERIAL: u16 = 0x3f8;
const DLL: u16 = 0;
const DLH: u16 = 1;
const IER: u16 = 1;
const FCR: u16 = 2;
const LCR: u16 = 3;
const LSR: u16 = 5;
const TX_READY: u8 = 0x20;

unsafe fn serial_write(ch: char) {
    while (inb(IOPORT_SERIAL + LSR) & TX_READY) == 0 {}
    outb(IOPORT_SERIAL, ch as u8);
}

pub fn printchar(ch: char) {
    unsafe {
        serial_write(ch);
        if ch == '\n' {
            serial_write('\r');
        }
    }
}

pub unsafe fn init() {
    let divisor: u16 = 12; // 115200 / 9600 = 12
    outb(IOPORT_SERIAL + IER, 0x00); // Disable interrupts.
    outb(IOPORT_SERIAL + DLL, (divisor & 0xff) as u8);
    outb(IOPORT_SERIAL + DLH, ((divisor >> 8) & 0xff) as u8);
    outb(IOPORT_SERIAL + LCR, 0x03); // 8n1.
    outb(IOPORT_SERIAL + FCR, 0x01); // Enable FIFO.
}

use x86::io::{inb, outb};

use crate::fs::devfs::{CONSOLE_FILE, DEV_FS};

use super::ioapic::enable_irq;

const IOPORT_SERIAL: u16 = 0x3f8;
const DLL: u16 = 0;
const RBR: u16 = 0;
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

pub fn print_str(s: &[u8]) {
    for ch in s {
        printchar(*ch as char);
    }
}

fn read_char() -> Option<char> {
    unsafe {
        if (inb(IOPORT_SERIAL + LSR) & 1) == 0 {
            return None;
        }

        Some(inb(IOPORT_SERIAL + RBR) as char)
    }
}

pub fn irq_handler() {
    while let Some(ch) = read_char() {
        if ch == '\r' {
            CONSOLE_FILE.input_char('\n');
        } else {
            CONSOLE_FILE.input_char(ch);
        }
    }
}

pub unsafe fn early_init() {
    let divisor: u16 = 12; // 115200 / 9600 = 12
    outb(IOPORT_SERIAL + IER, 0x00); // Disable interrupts.
    outb(IOPORT_SERIAL + DLL, (divisor & 0xff) as u8);
    outb(IOPORT_SERIAL + DLH, ((divisor >> 8) & 0xff) as u8);
    outb(IOPORT_SERIAL + LCR, 0x03); // 8n1.
    outb(IOPORT_SERIAL + FCR, 0x01); // Enable FIFO.
    outb(IOPORT_SERIAL + IER, 0x01); // Enable interrupts.
}

pub fn init() {
    enable_irq(4);
}

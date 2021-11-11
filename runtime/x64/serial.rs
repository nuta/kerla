use x86::io::{inb, outb};

use crate::{
    handler,
    print::{set_printer, Printer},
};

use super::{ioapic::enable_irq, vga};

pub const SERIAL_IRQ: u8 = 4;
const IOPORT_SERIAL: u16 = 0x3f8;
const DLL: u16 = 0;
const RBR: u16 = 0;
const DLH: u16 = 1;
const IER: u16 = 1;
const FCR: u16 = 2;
const LCR: u16 = 3;
const LSR: u16 = 5;
const TX_READY: u8 = 0x20;

unsafe fn serial_write(ch: u8) {
    while (inb(IOPORT_SERIAL + LSR) & TX_READY) == 0 {}
    outb(IOPORT_SERIAL, ch);
}

pub fn printchar(ch: u8) {
    unsafe {
        if ch == b'\n' && option_env!("DISABLE_AUTO_CR_PRINT").is_none() {
            serial_write(b'\r');
        }
        serial_write(ch);
    }
}

pub fn console_write(s: &[u8]) {
    for ch in s {
        printchar(*ch);
        vga::printchar(*ch);
    }
}

struct SerialPrinter;

impl Printer for SerialPrinter {
    fn print_bytes(&self, s: &[u8]) {
        console_write(s);
    }
}

fn read_char() -> Option<u8> {
    unsafe {
        if (inb(IOPORT_SERIAL + LSR) & 1) == 0 {
            return None;
        }

        Some(inb(IOPORT_SERIAL + RBR))
    }
}

pub fn irq_handler() {
    while let Some(ch) = read_char() {
        if ch == b'\r' {
            handler().handle_console_rx(b'\n');
        } else {
            handler().handle_console_rx(ch);
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

    set_printer(&SerialPrinter);
    printchar(b'\n');
}

pub fn init() {
    enable_irq(4);
}

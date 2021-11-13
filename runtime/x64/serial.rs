//! A serial port driver. See https://wiki.osdev.org/Serial_Ports
use x86::io::{inb, outb};

use crate::{
    handler,
    print::{set_debug_printer, set_printer, Printer},
};

use super::{ioapic::enable_irq, vga};

pub const SERIAL0_IOPORT: u16 = 0x3f8;
pub const SERIAL1_IOPORT: u16 = 0x2f8;
pub const SERIAL0_IRQ: u8 = 4;
pub const SERIAL1_IRQ: u8 = 3;
const THR: u16 = 0;
const DLL: u16 = 0;
const RBR: u16 = 0;
const DLH: u16 = 1;
const IER: u16 = 1;
const FCR: u16 = 2;
const LCR: u16 = 3;
const LSR: u16 = 5;
const TX_READY: u8 = 0x20;

struct SerialPort {
    ioport_base: u16,
    irq: u8,
}

impl SerialPort {
    pub const fn new(ioport_base: u16, irq: u8) -> SerialPort {
        SerialPort { ioport_base, irq }
    }

    pub fn initialize(&self) {
        let divisor: u16 = 12; // 115200 / 9600 = 12
        unsafe {
            self.outb(IER, 0x00); // Disable interrupts.
            self.outb(DLL, (divisor & 0xff) as u8);
            self.outb(DLH, ((divisor >> 8) & 0xff) as u8);
            self.outb(LCR, 0x03); // 8n1.
            self.outb(FCR, 0x01); // Enable FIFO.
            self.outb(IER, 0x01); // Enable interrupts.
        }
    }

    pub fn irq(&self) -> u8 {
        self.irq
    }

    pub fn print_char(&self, ch: u8) {
        if ch == b'\n' && option_env!("DISABLE_AUTO_CR_PRINT").is_none() {
            self.send_char(b'\r');
        }
        self.send_char(ch);
    }

    pub fn send_char(&self, ch: u8) {
        unsafe {
            while (self.inb(LSR) & TX_READY) == 0 {}
            self.outb(THR, ch);
        }
    }

    pub fn receive_char(&self) -> Option<u8> {
        unsafe {
            if (self.inb(LSR) & 1) == 0 {
                return None;
            }

            Some(self.inb(RBR))
        }
    }

    unsafe fn inb(&self, port: u16) -> u8 {
        inb(self.ioport_base + port)
    }

    unsafe fn outb(&self, port: u16, data: u8) {
        outb(self.ioport_base + port, data);
    }
}

static SERIAL0: SerialPort = SerialPort::new(SERIAL0_IOPORT, SERIAL0_IRQ);
static SERIAL1: SerialPort = SerialPort::new(SERIAL1_IOPORT, SERIAL1_IRQ);

struct Serial0Printer;

impl Printer for Serial0Printer {
    fn print_bytes(&self, s: &[u8]) {
        for ch in s {
            SERIAL0.print_char(*ch);
            vga::printchar(*ch);
        }
    }
}

struct Serial1Printer;

impl Printer for Serial1Printer {
    fn print_bytes(&self, s: &[u8]) {
        for ch in s {
            SERIAL1.print_char(*ch);
            vga::printchar(*ch);
        }
    }
}

pub fn serial0_irq_handler() {
    while let Some(ch) = SERIAL0.receive_char() {
        if ch == b'\r' {
            handler().handle_console_rx(b'\n');
        } else {
            handler().handle_console_rx(ch);
        }
    }
}

pub unsafe fn early_init() {
    SERIAL0.initialize();
    set_printer(&Serial0Printer);
    set_debug_printer(&Serial0Printer);

    SERIAL0.print_char(b'\n');
}

pub fn init(use_second_serialport: bool) {
    enable_irq(SERIAL0.irq());

    if use_second_serialport {
        SERIAL1.initialize();
        set_debug_printer(&Serial1Printer);
    }
}

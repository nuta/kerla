use crate::address::{PAddr, VAddr};

use core::ptr;

use kerla_utils::once::Once;
use x86::io::outb;

#[repr(u8)]
#[derive(Copy, Clone)]
#[allow(unused)]
enum VgaColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Purple = 5,
    Brown = 6,
    Gray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightPurple = 13,
    Yellow = 14,
    White = 15,
}

const COLUMNS: usize = 80;
const ROWS: usize = 25;

// Encoded in https://en.wikipedia.org/wiki/Code_page_437
const BANNER: &str =
    " Kerla       /dev/console is connected to the serial port (no keyboard support) ";

struct Console {
    base: VAddr,
    x: usize,
    y: usize,
    fg: VgaColor,
    bg: VgaColor,
}

impl Console {
    pub const fn new() -> Console {
        Console {
            base: PAddr::new(0xb8000).as_vaddr(),
            x: 0,
            y: 0,
            fg: VgaColor::White,
            bg: VgaColor::Black,
        }
    }

    pub fn clear_screen(&mut self) {
        self.x = 0;
        self.y = 0;

        for y in 0..ROWS {
            for x in 0..COLUMNS {
                self.draw_char(x, y, ' ', VgaColor::White, VgaColor::Black);
            }
        }

        self.move_cursor(0, 0);
        self.draw_banner();
    }

    fn move_cursor(&self, x: usize, y: usize) {
        unsafe {
            let pos = y * COLUMNS + x;
            outb(0x3d4, 0x0f);
            outb(0x3d5, (pos & 0xff) as u8);
            outb(0x3d4, 0x0e);
            outb(0x3d5, ((pos >> 8) & 0xff) as u8);
        }
    }

    unsafe fn mut_ptr_at(&self, x: usize, y: usize) -> *mut u16 {
        #[allow(clippy::ptr_offset_with_cast)]
        self.base
            .as_mut_ptr::<u16>()
            .offset((x + y * COLUMNS) as isize)
    }

    fn draw_char(&mut self, x: usize, y: usize, ch: char, fg: VgaColor, bg: VgaColor) {
        unsafe {
            self.mut_ptr_at(x, y)
                .write((((bg as u16) << 12) | (fg as u16) << 8) | (ch as u16));
        }
    }

    fn scroll(&mut self) {
        let diff = self.y - ROWS + 1;
        for from in diff..ROWS {
            unsafe {
                ptr::copy_nonoverlapping(
                    self.mut_ptr_at(0, from),
                    self.mut_ptr_at(0, from - diff),
                    COLUMNS,
                );
            }
        }

        // Clear the new lines.
        unsafe {
            ptr::write_bytes(self.mut_ptr_at(0, ROWS - diff), 0, COLUMNS);
        }

        self.y = ROWS - 1;
        self.draw_banner();
    }

    fn draw_banner(&mut self) {
        for (x, ch) in BANNER.chars().enumerate() {
            self.draw_char(x, 0, ch, VgaColor::Black, VgaColor::LightGreen);
        }
    }
}

impl vte::Perform for Console {
    fn execute(&mut self, byte: u8) {
        match byte {
            b'\r' => {
                self.x = 0;
            }
            b'\n' => {
                self.y += 1;
                self.x = 0;
            }
            _ => {}
        }

        if self.y >= ROWS {
            self.scroll();
        }

        self.move_cursor(self.x, self.y);
    }

    fn print(&mut self, c: char) {
        if self.x == COLUMNS {
            self.y += 1;
            self.x = 0;
        }

        if self.y > ROWS {
            self.scroll();
        }

        self.draw_char(self.x, self.y, c, self.fg, self.bg);
        self.x += 1;
        self.move_cursor(self.x, self.y);
    }
}

static TERMINAL_PARSER: Once<spin::Mutex<vte::Parser>> = Once::new();
static CONSOLE: spin::Mutex<Console> = spin::Mutex::new(Console::new());

pub fn printchar(ch: u8) {
    TERMINAL_PARSER.lock().advance(&mut *CONSOLE.lock(), ch);
}

pub fn init() {
    TERMINAL_PARSER.init(|| spin::Mutex::new(vte::Parser::new()));
    CONSOLE.lock().clear_screen();
}

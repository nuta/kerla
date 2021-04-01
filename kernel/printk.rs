use crate::arch::{print_str, printchar, Backtrace, VAddr};
use core::mem::size_of;
use core::slice;
use core::str;
pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        printchar(c);
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print_str(s.as_bytes());
        Ok(())
    }
}

/// Prints a string.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        #![allow(unused_import)]
        use core::fmt::Write;
        write!($crate::printk::Printer, "{}", format_args!($($arg)*)).ok();
    }};
}

/// Prints a string and a newline.
#[macro_export]
macro_rules! println {
    ($fmt:expr) => { $crate::print!(concat!($fmt, "\n")); };
    ($fmt:expr, $($arg:tt)*) => { $crate::print!(concat!($fmt, "\n"), $($arg)*); };
}

/// Prints a warning message only in the debug build.
#[macro_export]
macro_rules! debug_warn {
    ($fmt:expr) => {
        #[cfg(debug_assertions)]
        $crate::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"));
    };
    ($fmt:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"), $($arg)*);
    };
}

/// Prints a warning message if it is `Err`.
#[macro_export]
macro_rules! warn_if_err {
    ($result:expr) => {
        #[cfg(debug_assertions)]
        if let Err(err) = $result {
            $crate::debug_warn!("{}:{}: error returned: {:?}", file!(), line!(), err)
        }
    };
}

pub struct PrintkLogger;
impl log::Log for PrintkLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if cfg!(debug_assertions) {
            true
        } else {
            metadata.level() <= log::Level::Info
        }
    }

    fn log(&self, record: &log::Record) {
        use log::Level;
        const RESET: &str = "\x1b[0m";
        const INFO_COLOR: &str = "\x1b[36m";
        const WARN_COLOR: &str = "\x1b[33m";
        const ERROR_COLOR: &str = "\x1b[1;31m";

        if self.enabled(record.metadata()) {
            match record.level() {
                Level::Trace | Level::Debug => {
                    println!("{}", record.args());
                }
                Level::Info => {
                    println!("{}{}{}", INFO_COLOR, record.args(), RESET);
                }
                Level::Warn => {
                    println!("{}{}{}", WARN_COLOR, record.args(), RESET);
                }
                Level::Error => {
                    println!("{}{}{}", ERROR_COLOR, record.args(), RESET);
                }
            }
        }
    }

    fn flush(&self) {}
}

/// A symbol.
#[repr(C, packed)]
struct SymbolEntry {
    addr: u64,
    name: [u8; 56],
}

#[repr(C, packed)]
struct SymbolTable {
    magic: u32,
    num_symbols: i32,
    padding: u64,
}

extern "C" {
    static __symbol_table: SymbolTable;
}

global_asm!(
    r#"
    .rodata
    .align 8
    .global __symbol_table
    __symbol_table:
       .ascii "__SYMBOL_TABLE_START__"
       .space 512 * 1024
       .ascii "__SYMBOL_TABLE_END__"
"#
);

struct Symbol {
    name: &'static str,
    addr: VAddr,
}

fn resolve_symbol(vaddr: VAddr) -> Option<Symbol> {
    assert!(unsafe { __symbol_table.magic } == 0xbeefbeef);

    let num_symbols = unsafe { __symbol_table.num_symbols };
    let symbols = unsafe {
        slice::from_raw_parts(
            ((&__symbol_table as *const _ as usize) + size_of::<SymbolTable>())
                as *const SymbolEntry,
            __symbol_table.num_symbols as usize,
        )
    };

    // Do a binary search.
    let mut l = -1;
    let mut r = num_symbols;
    while r - l > 1 {
        let mid = (l + r) / 2;
        if vaddr.value() >= symbols[mid as usize].addr as usize {
            l = mid;
        } else {
            r = mid;
        }
    }

    if l >= 0 {
        let symbol = &symbols[l as usize];
        Some(Symbol {
            name: unsafe { str::from_utf8_unchecked(&symbol.name) },
            addr: VAddr::new(symbol.addr as usize),
        })
    } else {
        None
    }
}

/// Prints a backtrace.
pub fn backtrace() {
    Backtrace::current_frame().traverse(|i, vaddr| {
        if let Some(symbol) = resolve_symbol(vaddr) {
            warn!(
                "    {index}: {vaddr} {symbol_name}()+0x{offset:x}",
                index = i,
                vaddr = vaddr,
                symbol_name = symbol.name,
                offset = vaddr.value() - symbol.addr.value(),
            );
        } else {
            warn!(
                "    {index}: {vaddr} (symbol unknown)",
                index = i,
                vaddr = vaddr,
            );
        }
    });
}

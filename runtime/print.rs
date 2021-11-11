use core::{fmt, str};

use kerla_utils::static_cell::StaticCell;

static PRINTER: StaticCell<&dyn Printer> = StaticCell::new(&NopPrinter);

/// Sets the global log printer.
pub fn set_printer(new_printer: &'static dyn Printer) {
    PRINTER.store(new_printer);
}

pub trait Printer: Sync {
    fn print_str(&self, s: &str) {
        self.print_bytes(s.as_bytes());
    }

    fn print_bytes(&self, s: &[u8]);
}

struct NopPrinter;

impl Printer for NopPrinter {
    fn print_bytes(&self, _s: &[u8]) {
        // Because the panic handler cannot use the printer, we have no way
        // to print a message. Use a debugger to check whether CPU reached here.
    }
}

/// A private struct internally used in print macros. Don't use this!
pub struct PrinterWrapper;

impl fmt::Write for PrinterWrapper {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        print_bytes(s.as_bytes());
        Ok(())
    }
}

pub fn print_bytes(s: &[u8]) {
    PRINTER.load().print_bytes(s);
}

/// Prints a string.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        #![allow(unused_imports)]
        use core::fmt::Write;
        write!($crate::print::PrinterWrapper, "{}", format_args!($($arg)*)).ok();
    }};
}

/// Prints a string and a newline.
#[macro_export]
macro_rules! println {
    () => {{
        $crate::print!(
            ""
        );
    }};
    ($fmt:expr) => {{
        $crate::print!(
            $fmt
        );
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::print!(
            concat!( $fmt, "\n"),
            $($arg)*
        );
    }};
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

/// Prints a warning message only once.
#[macro_export]
macro_rules! warn_once {
    ($fmt:expr) => {{
        static ONCE: ::spin::Once<()> = ::spin::Once::new();
        ONCE.call_once(|| {
            $crate::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"));
        });
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        static ONCE: ::spin::Once<()> = ::spin::Once::new();
        ONCE.call_once(|| {
            $crate::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"), $($arg)*);
        });
    }};
}

/// Prints a warning message if it is `Err`.
#[macro_export]
macro_rules! warn_if_err {
    ($result:expr) => {
        #[cfg(debug_assertions)]
        if let Err(err) = $result {
            $crate::debug_warn!("{}:{}: error returned: {:?}", file!(), line!(), err);
        }
    };
}

pub struct PrintkPrinter;
impl log::Log for PrintkPrinter {
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

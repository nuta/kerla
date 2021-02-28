use crate::arch::x64::printchar;

pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        printchar(c);
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.chars() {
            printchar(ch);
        }
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

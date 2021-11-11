use kerla_runtime::arch::console_write;
use kerla_runtime::print::{set_printer, Printer};
use kerla_utils::ring_buffer::RingBuffer;

use crate::lang_items::PANICKED;
use core::sync::atomic::Ordering;

pub struct LoggedPrinter;

pub const KERNEL_LOG_BUF_SIZE: usize = 8192;
// We use spin::Mutex here because SpinLock's debugging features may cause a
// problem (capturing a backtrace requires memory allocation).
pub static KERNEL_LOG_BUF: spin::Mutex<RingBuffer<u8, KERNEL_LOG_BUF_SIZE>> =
    spin::Mutex::new(RingBuffer::new());

impl Printer for LoggedPrinter {
    fn print_bytes(&self, s: &[u8]) {
        console_write(s);

        // Don't write into the kernel log buffer as it may call a printk function
        // due to an assertion.
        if !PANICKED.load(Ordering::SeqCst) {
            KERNEL_LOG_BUF.lock().push_slice(s);
        }
    }
}

/// Prints a warning message only in the debug build.
#[macro_export]
macro_rules! debug_warn {
    ($fmt:expr) => {
        if cfg!(debug_assertions) {
            ::kerla_runtime::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"));
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        if cfg!(debug_assertions) {
            ::kerla_runtime::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"), $($arg)*);
        }
    };
}

/// Prints a warning message only once.
#[macro_export]
macro_rules! warn_once {
    ($fmt:expr) => {{
        static ONCE: ::spin::Once<()> = ::spin::Once::new();
        ONCE.call_once(|| {
            ::kerla_runtime::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"));
        });
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        static ONCE: ::spin::Once<()> = ::spin::Once::new();
        ONCE.call_once(|| {
            ::kerla_runtime::println!(concat!("\x1b[1;33mWARN: ", $fmt, "\x1b[0m"), $($arg)*);
        });
    }};
}

/// Prints a warning message if it is `Err`.
#[macro_export]
macro_rules! warn_if_err {
    ($result:expr) => {
        if cfg!(debug_assertions) {
            if let Err(err) = $result {
                $crate::debug_warn!("{}:{}: error returned: {:?}", file!(), line!(), err);
            }
        }
    };
}

pub fn init() {
    set_printer(&LoggedPrinter);
}

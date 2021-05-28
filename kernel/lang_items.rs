use core::sync::atomic::{AtomicBool, Ordering};

pub static PANICKED: AtomicBool = AtomicBool::new(false);

/// This function is called on panic.
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    PANICKED.store(true, Ordering::SeqCst);
    crate::printk::backtrace();
    loop {
        crate::arch::halt();
    }
}

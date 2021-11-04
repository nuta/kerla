use core::sync::atomic::AtomicBool;

pub static PANICKED: AtomicBool = AtomicBool::new(false);

/// This function is called on panic.
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::sync::atomic::Ordering;

    PANICKED.store(true, Ordering::SeqCst);
    error!("{}", info);
    crate::printk::backtrace();
    crate::arch::halt();
}

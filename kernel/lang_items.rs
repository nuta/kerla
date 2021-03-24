/// This function is called on panic.
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    crate::printk::backtrace();
    loop {
        crate::arch::halt();
    }
}

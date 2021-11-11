use core::sync::atomic::AtomicBool;

pub static PANICKED: AtomicBool = AtomicBool::new(false);

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("alloc error: layout={:?}", layout);
}

/// This function is called on panic.
#[panic_handler]
#[cfg(not(test))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::sync::atomic::Ordering;

    if PANICKED.load(Ordering::SeqCst) {
        kerla_runtime::print::print_bytes(b"double panic!\n");
        kerla_runtime::arch::halt();
    }

    PANICKED.store(true, Ordering::SeqCst);
    error!("{}", info);
    kerla_runtime::backtrace::backtrace();
    kerla_runtime::arch::halt();
}

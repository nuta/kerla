#![cfg(test)]
#![allow(clippy::print_with_newline)]

use core::panic::PanicInfo;
use core::sync::atomic::Ordering;

use kerla_runtime::arch::{semihosting_halt, SemihostingExitStatus};

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{} ... ", core::any::type_name::<T>());
        self();
        print!("\x1b[92mok\x1b[0m\n");
    }
}

pub fn run_tests(tests: &[&dyn Testable]) {
    println!("Running {} tests\n", tests.len());
    for test in tests {
        test.run();
    }
    print!("\n");
    print!("\x1b[92mPassed all tests :)\x1b[0m\n");
}

pub fn end_tests() -> ! {
    semihosting_halt(SemihostingExitStatus::Success);

    #[allow(clippy::empty_loop)]
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::lang_items::PANICKED;

    if PANICKED.load(Ordering::SeqCst) {
        loop {}
    }

    PANICKED.store(true, Ordering::SeqCst);
    print!("\x1b[1;91mfail\npanic: {}\x1b[0m", info);
    semihosting_halt(SemihostingExitStatus::Failure);
    loop {}
}

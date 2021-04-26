#![cfg(test)]

use crate::arch::*;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

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
    println!("\x1b[92mPassed all tests :)\x1b[0m\n");
}

pub fn end_tests() -> ! {
    semihosting_halt(ExitStatus::Success);

    #[allow(clippy::empty_loop)]
    loop {}
}

static ALREADY_PANICED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if ALREADY_PANICED.load(Ordering::SeqCst) {
        loop {}
    }

    ALREADY_PANICED.store(true, Ordering::SeqCst);

    print!("\x1b[1;91mfail\x1b[0m\n{}\n", info);
    semihosting_halt(ExitStatus::Failure);
    loop {}
}

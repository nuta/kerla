#![cfg(test)]

use crate::arch::*;
use core::panic::PanicInfo;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{} ... ", core::any::type_name::<T>());
        self();
        println!("\x1b[92mok\x1b[0m");
    }
}

pub fn run_tests(tests: &[&dyn Testable]) {
    println!("Running {} tests\n", tests.len());
    for test in tests {
        test.run();
    }
    println!("\n\x1b[92mPassed all tests :)\x1b[0m");
}

pub fn end_tests() -> ! {
    semihosting_halt(ExitStatus::Success);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\x1b[1;91mfail\x1b[0m\n{}", info);
    semihosting_halt(ExitStatus::Failure);
    loop {}
}

#[test_case]
fn one_plus_two_equals_three() {
    assert_eq!(1 + 2, 3);
}

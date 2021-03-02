#![cfg_attr(test, allow(unreachable_code))]

use crate::arch::idle;
#[cfg(test)]
use crate::test_runner::end_tests;

pub fn boot_kernel() {
    #[cfg(test)]
    {
        crate::test_main();
        end_tests();
    }

    println!("Hello World from Penguin Kernel XD");
    loop {
        idle();
    }
}

#![cfg_attr(test, allow(unreachable_code))]

use crate::allocator;
use crate::arch::{idle, PAddr};
#[cfg(test)]
use crate::test_runner::end_tests;
use arrayvec::ArrayVec;

pub struct RamArea {
    pub base: PAddr,
    pub len: usize,
}

pub struct BootInfo {
    pub ram_areas: ArrayVec<[RamArea; 8]>,
}

pub fn boot_kernel(bootinfo: &BootInfo) {
    allocator::init(&bootinfo.ram_areas);

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

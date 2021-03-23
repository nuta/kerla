#![cfg_attr(test, allow(unreachable_code))]

use crate::arch::{idle, PAddr};
use crate::mm::{global_allocator, page_allocator};

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
    page_allocator::init(&bootinfo.ram_areas);
    global_allocator::init();

    #[cfg(test)]
    {
        crate::test_main();
        end_tests();
    }

    println!("Hello World from Penguin Kernel XD");
    crate::arch::init();
    crate::fs::devfs::init();
    crate::fs::initramfs::init();
    crate::process::init();

    loop {
        idle();
    }
}

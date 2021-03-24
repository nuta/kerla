#![cfg_attr(test, allow(unreachable_code))]

use crate::mm::{global_allocator, page_allocator};
use crate::{
    arch::{idle, PAddr},
    printk::PrintkLogger,
};

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

static LOGGER: PrintkLogger = PrintkLogger;

pub fn init_logger() {
    log::set_logger(&PrintkLogger).unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });
}

pub fn boot_kernel(bootinfo: &BootInfo) {
    page_allocator::init(&bootinfo.ram_areas);
    global_allocator::init();

    #[cfg(test)]
    {
        crate::test_main();
        end_tests();
    }

    info!("Hello World from Penguin Kernel XD");
    crate::arch::init();
    crate::fs::devfs::init();
    crate::fs::initramfs::init();
    crate::process::init();

    loop {
        idle();
    }
}

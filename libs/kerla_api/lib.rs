#![no_std]

use kerla_runtime::bootinfo::VirtioMmioDevice;

extern crate alloc;

#[macro_use]
extern crate log;

pub mod driver;

pub use kerla_runtime::{debug_warn, warn_if_err, warn_once};
pub use log::{debug, error, info, trace, warn};

pub mod address {
    pub use kerla_runtime::address::{PAddr, VAddr};
}

pub mod mm {
    pub use kerla_runtime::page_allocator::{alloc_pages, AllocPageFlags, PageAllocError};
}

pub mod sync {
    pub use kerla_runtime::spinlock::{SpinLock, SpinLockGuard};
}

pub mod arch {
    pub use kerla_runtime::arch::PAGE_SIZE;
}

pub fn init() {}
pub fn init_pci_devices() {}
pub fn init_virtio_mmio_devices(devices: &[VirtioMmioDevice]) {}

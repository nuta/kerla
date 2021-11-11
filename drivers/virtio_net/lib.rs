#![no_std]

extern crate alloc;

#[macro_use]
extern crate kerla_api;

use kerla_api::bootinfo::VirtioMmioDevice;

use super::DRIVER_BUILDERS;

pub mod transports;
pub mod virtio;
pub mod virtio_net;

pub fn init(mmio_devices: &[VirtioMmioDevice]) {
    for device in mmio_devices {
        for builder in DRIVER_BUILDERS.lock().iter() {
            builder.attach_virtio_mmio(device).ok();
        }
    }
}

pub fn init() {
    // Initialize the array of all device drivers.
    DRIVER_BUILDERS
        .lock()
        .push(Box::new(VirtioNetBuilder::new()));
}

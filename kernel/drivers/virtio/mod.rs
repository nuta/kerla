use kerla_runtime::bootinfo::VirtioMmioDevice;

use super::DRIVER_BUILDERS;

pub mod transports;
#[allow(clippy::module_inception)]
pub mod virtio;
pub mod virtio_net;

pub fn init(mmio_devices: &[VirtioMmioDevice]) {
    for device in mmio_devices {
        for builder in DRIVER_BUILDERS.lock().iter() {
            builder.attach_virtio_mmio(device).ok();
        }
    }
}

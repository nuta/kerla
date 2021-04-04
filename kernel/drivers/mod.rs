use crate::arch::SpinLock;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub mod driver;
pub mod ioport;
pub mod pci;
pub mod virtio;
pub mod virtio_net;

pub use driver::*;

use alloc::boxed::Box;
use virtio_net::VirtioNetBuilder;

pub(super) static DRIVER_BUILDERS: SpinLock<Vec<Box<dyn DriverBuilder>>> =
    SpinLock::new(Vec::new());

/// Activated ethernet device drivers.
static ETHERNET_DRIVERS: SpinLock<Vec<Arc<SpinLock<dyn EthernetDriver>>>> =
    SpinLock::new(Vec::new());

pub fn register_ethernet_driver(driver: Arc<SpinLock<dyn EthernetDriver>>) {
    ETHERNET_DRIVERS.lock().push(driver);
}

pub fn get_ethernet_driver() -> Option<Arc<SpinLock<dyn EthernetDriver>>> {
    ETHERNET_DRIVERS.lock().get(0).cloned()
}

pub fn init() {
    // Initialize the array of all device drivers.
    DRIVER_BUILDERS
        .lock()
        .push(Box::new(VirtioNetBuilder::new()));

    // Scan PCI devices.
    for device in pci::enumerate_pci_devices() {
        trace!(
            "pci: found a device: id={:04x}:{:04x}, bar0={:016x?}, irq={}",
            device.config().vendor_id(),
            device.config().device_id(),
            device.config().bar0(),
            device.config().interrupt_line()
        );

        for builder in DRIVER_BUILDERS.lock().iter() {
            builder.attach_pci(&device).ok();
        }
    }
}

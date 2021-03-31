use alloc::vec::Vec;
use alloc::{collections::BTreeMap, sync::Arc};

pub mod driver;
pub mod ioport;
pub mod pci;
pub mod virtio;
pub mod virtio_net;

pub use driver::*;
use hashbrown::HashMap;

use crate::{
    arch::{enable_irq, SpinLock},
    net::iterate_event_loop,
};
use alloc::boxed::Box;
use core::any::Any;
use virtio_net::{VirtioNet, VirtioNetBuilder};

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

// TODO: Use a simple array for faster access.
static IRQ_HANDLERS: SpinLock<BTreeMap<u8, Box<dyn FnMut() + Send + Sync>>> =
    SpinLock::new(BTreeMap::new());

pub fn attach_irq<F: FnMut() + Send + Sync + 'static>(vec: u8, f: F) {
    IRQ_HANDLERS.lock().insert(vec, Box::new(f));
    enable_irq(vec);
}

pub fn handle_irq(vec: u8) {
    IRQ_HANDLERS
        .lock()
        .get_mut(&vec)
        .map(|handler| (*handler)());

    // FIXME:
    // We need to release the driver lock.
    iterate_event_loop();
}

pub fn init() {
    // Initialize the array of all device drivers.
    DRIVER_BUILDERS
        .lock()
        .push(Box::new(VirtioNetBuilder::new()));

    // Scan PCI devices.
    for device in pci::enumerate_pci_devices() {
        unsafe {
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
}

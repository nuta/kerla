use crate::arch::SpinLock;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub mod driver;
pub mod ioport;
pub mod pci;
pub mod virtio;

pub use driver::*;

use alloc::boxed::Box;

use self::virtio::virtio_net::VirtioNetBuilder;

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
}

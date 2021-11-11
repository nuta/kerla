use self::pci::PciDevice;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub mod ioport;
pub mod pci;

pub use kerla_runtime::bootinfo::VirtioMmioDevice;

use alloc::boxed::Box;
use kerla_runtime::spinlock::SpinLock;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub fn new(addr: [u8; 6]) -> MacAddress {
        MacAddress(addr)
    }
    pub fn as_array(&self) -> [u8; 6] {
        self.0
    }
}

pub trait Driver: Send + Sync {
    fn name(&self) -> &str;
}

pub trait EthernetDriver: Driver {
    fn mac_addr(&self) -> MacAddress;
    fn transmit(&mut self, frame: &[u8]);
}

pub trait DriverBuilder: Send + Sync {
    fn attach_pci(&self, pci_device: &PciDevice);
    fn attach_virtio_mmio(&self, mmio_device: &VirtioMmioDevice);
}

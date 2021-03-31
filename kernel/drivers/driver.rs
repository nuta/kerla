use super::pci::PciDevice;
use crate::arch::SpinLock;
use crate::result::{Errno, Error, Result};
use alloc::boxed::Box;
use core::any::Any;

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

pub trait Driver: Send + Sync + Any {
    fn name(&self) -> &str;
}

pub trait EthernetDriver: Driver {
    fn mac_addr(&self) -> Result<MacAddress>;
    fn transmit(&mut self, frame: &[u8]) -> Result<()>;
}

pub trait DriverBuilder: Send + Sync {
    fn attach_pci(&self, pci_device: &PciDevice) -> Result<()>;
}

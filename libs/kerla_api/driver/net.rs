use alloc::boxed::Box;

use super::Driver;

use crate::kernel_ops::kernel_ops;

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

pub trait EthernetDriver: Driver {
    fn mac_addr(&self) -> MacAddress;
    fn transmit(&self, frame: &[u8]);
}

pub fn register_ethernet_driver(driver: Box<dyn EthernetDriver>) {
    kernel_ops().register_ethernet_driver(driver);
}

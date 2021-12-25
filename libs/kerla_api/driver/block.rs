use alloc::boxed::Box;

use crate::{driver::Driver, kernel_ops::kernel_ops};

pub trait BlockDriver: Driver {
    fn read_block(&self, sector: u64, buf: &mut [u8]);
    fn write_block(&self, sector: u64, buf: &[u8]);
}

pub fn register_block_driver(driver: Box<dyn BlockDriver>) {
    kernel_ops().register_block_driver(driver)
}

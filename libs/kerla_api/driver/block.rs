use crate::driver::Driver;

pub trait BlockDriver: Driver {
    fn read_block(&self, sector: u64, frame: &[u8]);
    fn write_block(&self, sector: u64, frame: &[u8]);
}
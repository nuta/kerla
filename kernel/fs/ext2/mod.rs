#![cfg_attr(not(test), no_std)]

use crate::result::{Error, Result};
use core::ops::{Deref, DerefMut};

pub trait Disk {
    fn read_block(&mut self, lba: usize, buf: &mut [u8]) -> Result<usize>;
    fn write_block(&mut self, lba: usize, buf: &[u8]) -> Result<usize>;
}

pub struct Ext2Fs<D: Disk> {
    disk: D,
}

impl<D: Disk> Ext2Fs<D> {
    pub fn new(disk: D) -> Ext2Fs<D> {
        Ext2Fs { disk }
    }
}

impl<D: Disk> Deref for Ext2Fs<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.disk
    }
}

impl<D: Disk> DerefMut for Ext2Fs<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.disk
    }
}

#[cfg(test)]
mod tests {
    use crate::fs::ext2::{Disk, Ext2Fs};
    use crate::result::{Error, Result};
    use alloc::vec::Vec;

    struct PseudoDisk {
        mem_disk: Vec<u8>,
    }

    impl PseudoDisk {
        /// memory-based disk block with a size of 1MB
        const DEFAULT_DISK_SIZE: usize = 1024 * 1024;

        pub fn new() -> PseudoDisk {
            PseudoDisk {
                mem_disk: vec![0u8; PseudoDisk::DEFAULT_DISK_SIZE],
            }
        }
    }

    impl Disk for PseudoDisk {
        fn read_block(&mut self, lba: usize, buf: &mut [u8]) -> crate::prelude::Result<usize> {
            let disk_data = &self.mem_disk;

            let mut real_read_len = buf.len();
            if lba + buf.len() > disk_data.len() {
                real_read_len = disk_data.len() - lba;
            }
            buf.copy_from_slice(&disk_data[lba..lba + real_read_len]);
            Ok(real_read_len)
        }

        fn write_block(&mut self, lba: usize, buf: &[u8]) -> crate::prelude::Result<usize> {
            let mut disk_data = &self.mem_disk;
            let write_data = Vec::from(buf);

            let mut write_len = buf.len();
            if buf.len() + lba > disk_data.len() {
                write_len = disk_data.len() - lba;
            }
            self.mem_disk.splice(lba..lba + write_len, write_data);
            Ok(write_len)
        }
    }

    struct Test {
        data: Vec<u8>,
        lba: usize,
        write_len: usize,
        read_len: usize,
        read_result: Vec<u8>,
    }

    #[test]
    fn test() -> Result<()> {
        let tests = vec![
            Test {
                data: vec![1u8, 2, 3, 4, 5],
                lba: 0,
                write_len: 5,
                read_len: 5,
                read_result: vec![1u8, 2, 3, 4, 5],
            },
            Test {
                data: vec![2u8, 12, 32, 32, 32],
                lba: 1024 * 1024 - 3,
                write_len: 3,
                read_len: 3,
                read_result: vec![2u8, 12, 32],
            },
        ];
        let mut pseudo_disk = PseudoDisk::new();
        let mut ext2 = Ext2Fs::new(pseudo_disk);

        for test in tests {
            let lba = test.lba;
            let data = test.data;

            // check write
            let write_len = ext2.write_block(lba.clone(), data.as_slice())?;
            assert_eq!(write_len, test.write_len);

            // check read
            let mut read_data = vec![0u8; test.read_result.len()];
            let mut read_vec = read_data.as_slice();
            let read_len = ext2.read_block(test.lba, &mut read_vec)?;
            assert_eq!(read_len, test.read_len);

            // check result
            assert_eq!(read_vec, test.read_result)
        }

        Ok(())
    }
}

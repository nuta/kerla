#[derive(Debug, PartialEq)]
pub enum DiskError {
    EOF,
    IOException
}

pub trait Disk {
    fn read_block(&mut self, lba: usize, buf: &mut [u8]) -> Result<usize, DiskError>;
    fn write_block(&mut self, lba: usize, buf: &[u8]) -> Result<usize, DiskError>;
}

pub struct Ext2Fs<D: Disk> {
    disk: D,
}

impl<D: Disk> Ext2Fs<D> {
    pub fn new(disk: D) -> Ext2Fs<D> {
        Ext2Fs { disk }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use crate::ext2::{Disk, DiskError, Ext2Fs};

    struct PseudoDisk {
        mem_disk: Vec<u8>,
    }

    impl PseudoDisk {
        /// memory-based disk block with a size of 1KB
        const DEFAULT_DISK_SIZE: usize = 1024;

        pub fn new() -> PseudoDisk {
            PseudoDisk {
                mem_disk: vec![0u8; PseudoDisk::DEFAULT_DISK_SIZE],
            }
        }
    }

    impl Disk for PseudoDisk {
        fn read_block(&mut self, lba: usize, buf: &mut [u8]) -> Result<usize, DiskError> {
            let disk_data = &self.mem_disk;

            let mut real_read_len = buf.len();
            if lba + buf.len() > disk_data.len() {
                real_read_len = disk_data.len() - lba;
            }
            buf.copy_from_slice(&disk_data[lba..lba + real_read_len]);
            Ok(real_read_len)
        }

        fn write_block(&mut self, lba: usize, buf: &[u8]) -> Result<usize, DiskError> {
            let disk_data = &self.mem_disk;
            let write_data = Vec::from(buf);

            let mut write_len = buf.len();
            if buf.len() + lba > disk_data.len() {
                write_len = disk_data.len() - lba;
            }
            self.mem_disk.splice(lba..lba + write_len, write_data);
            Ok(write_len)
        }
    }

    #[derive(PartialEq)]
    struct Test {
        data: Vec<u8>,
        lba: usize,
        write_len: usize,
        read_len: usize,
        read_result: Vec<u8>,
    }

    #[test]
    fn test() {
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
                lba: 1024 - 3,
                write_len: 3,
                read_len: 3,
                read_result: vec![2u8, 12, 32],
            },
        ];
        let pseudo_disk = PseudoDisk::new();
        let mut ext2 = Ext2Fs::new(pseudo_disk);

        for test in tests {
            let lba = test.lba;
            let data = test.data;
            // check write
            let write_len = ext2.disk.write_block(lba, data.as_slice());
            match write_len {
                Ok(len) => {
                    assert_eq!(len, test.write_len);
                }
                Err(_) => {
                    assert!(false)
                }
            }

            // check read
            let mut read_data = vec![0u8; test.read_result.len()];
            let read_len = ext2.disk.read_block(test.lba, &mut read_data);
            match read_len {
                Ok(len) => {
                    assert_eq!(len, test.read_len);
                }
                Err(_) => {
                    assert!(false)
                }
            }

            // check result
            assert_eq!(read_data, test.read_result)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::OpenOptions, io::{BufWriter, Read, Seek, SeekFrom}};

    use ext2_fs::super_block;

    #[test]
    fn test_read_super_block() {
        let mut image_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("./ramdisk.image").unwrap();

        let mut block_data = [0u8; 2048];
        image_file.seek(SeekFrom::Start(0)).unwrap();
        image_file.read_exact(&mut block_data).unwrap();

        let sb = super_block::ext2_fill_super(&block_data).unwrap();
        println!("{:?}", sb);
    }

}

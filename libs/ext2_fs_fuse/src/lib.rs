#[cfg(test)]
mod tests {
    use std::{fs::OpenOptions, io::{Read, Seek, SeekFrom}};
    use ext2_fs::read_super_block;

    #[test]
    fn test_read_super_block() {
        let mut image_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("./ramdisk.image").unwrap();

        let mut block_data = [0u8; 1024];
        image_file.seek(SeekFrom::Start(1024)).unwrap();
        image_file.read_exact(&mut block_data).unwrap();

        let sb = read_super_block(&block_data).unwrap();
        println!("{:?}", sb);
    }

}

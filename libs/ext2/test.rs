#[cfg(test)]
mod test {
    use std::fs::{create_dir, OpenOptions};
    use std::io::{BufReader, Read};
    use std::println;
    use crate::ext2::Ext2SuperBlock;
    use crate::super_block;

    #[test]
    fn test() {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("img/ramdisk.image").unwrap();
        let mut buf_reader = BufReader::new(f);
        let mut blocks = [0u8; 2048];
        buf_reader.read_exact(&mut blocks);
        let sb_block = super_block::ext2_fill_super(&blocks).unwrap();

        assert_eq!(&0xef53, sb_block.get_magic());
        assert_eq!(&16, sb_block.get_inode_count());
        assert_eq!(&1024, sb_block.get_block_count());
        assert_eq!(&51, sb_block.get_r_block_count())
    }

}

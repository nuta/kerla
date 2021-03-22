use crate::fs::inode::INodeNo;

pub const S_IFMT: u32 = 0o170000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFREG: u32 = 0o100000;

#[derive(Debug, Copy, Clone)]
pub struct FileMode(u32);

impl FileMode {
    pub fn new(value: u32) -> FileMode {
        FileMode(value)
    }

    pub fn is_directory(self) -> bool {
        (self.0 & S_IFMT) == S_IFDIR
    }

    pub fn is_regular_file(self) -> bool {
        (self.0 & S_IFMT) == S_IFREG
    }
}

#[derive(Debug, Clone)]
pub struct Stat {
    pub inode_no: INodeNo,
    pub mode: FileMode,
}

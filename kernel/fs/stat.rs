use crate::fs::inode::INodeNo;

/// The device file's ID.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct DevId(usize);

/// The number of hard links.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct NLink(usize);

/// The file size in bytes.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct FileSize(pub isize);

/// The user ID.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct UId(u32);

/// The Group ID.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct GId(u32);

/// The size in bytes of a block file file system I/O operations.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct BlockSize(isize);

/// The number of blocks.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct BlockCount(isize);

/// The file size in bytes.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Time(isize);

pub const S_IFMT: u32 = 0o170000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFLNK: u32 = 0o120000;

pub const O_ACCMODE: u32 = 0o3;

// FIXME: OpenFlags also define these values.
#[allow(unused)]
pub const O_RDONLY: u32 = 0o0;
pub const O_WRONLY: u32 = 0o1;
pub const O_RDWR: u32 = 0o2;

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct FileMode(u32);

impl FileMode {
    pub fn new(value: u32) -> FileMode {
        FileMode(value)
    }

    pub fn access_mode(self) -> u32 {
        self.0 & O_ACCMODE
    }

    pub fn is_directory(self) -> bool {
        (self.0 & S_IFMT) == S_IFDIR
    }

    pub fn is_regular_file(self) -> bool {
        (self.0 & S_IFMT) == S_IFREG
    }

    pub fn is_symbolic_link(self) -> bool {
        (self.0 & S_IFMT) == S_IFLNK
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct Stat {
    pub dev: DevId,
    pub inode_no: INodeNo,
    pub nlink: NLink,
    pub mode: FileMode,
    pub uid: UId,
    pub gid: GId,
    pub pad0: u32,
    pub rdev: DevId,
    pub size: FileSize,
    pub blksize: BlockSize,
    pub blocks: BlockCount,
    pub atime: Time,
    pub mtime: Time,
    pub ctime: Time,
}

impl Stat {
    pub fn zeroed() -> Stat {
        Stat {
            dev: DevId(0),
            inode_no: INodeNo::new(0),
            mode: FileMode(0),
            nlink: NLink(0),
            uid: UId(0),
            gid: GId(0),
            pad0: 0,
            rdev: DevId(0),
            size: FileSize(0),
            blksize: BlockSize(0),
            blocks: BlockCount(0),
            atime: Time(0),
            mtime: Time(0),
            ctime: Time(0),
        }
    }
}

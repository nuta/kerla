use super::{
    file_system::FileSystem,
    inode::{DirEntry, Directory, FileLike, INode, INodeNo},
    path::PathBuf,
    stat::{FileMode, Stat, S_IFDIR},
};
use crate::{
    arch::print_str,
    result::{Errno, Error, Result},
};
use alloc::sync::Arc;
use penguin_utils::once::Once;

static ROOT_DIR: Once<Arc<dyn Directory>> = Once::new();
static NULL_FILE: Once<Arc<dyn FileLike>> = Once::new();
static CONSOLE_FILE: Once<Arc<dyn FileLike>> = Once::new();

pub static DEV_FS: Once<Arc<DevFs>> = Once::new();

pub struct DevFs {}

impl DevFs {
    pub fn new() -> DevFs {
        ROOT_DIR.init(|| Arc::new(DevRootDir::new()) as Arc<dyn Directory>);
        NULL_FILE.init(|| Arc::new(NullFile::new()) as Arc<dyn FileLike>);
        CONSOLE_FILE.init(|| Arc::new(ConsoleFile::new()) as Arc<dyn FileLike>);
        DevFs {}
    }
}

impl FileSystem for DevFs {
    fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        Ok(ROOT_DIR.clone())
    }
}

/// The `/dev` directory.
struct DevRootDir {}
impl DevRootDir {
    pub fn new() -> DevRootDir {
        DevRootDir {}
    }
}

impl Directory for DevRootDir {
    fn lookup(&self, name: &str) -> Result<DirEntry> {
        match name {
            "null" => Ok(DirEntry {
                inode: INode::FileLike(NULL_FILE.clone()),
            }),
            "console" => Ok(DirEntry {
                inode: INode::FileLike(CONSOLE_FILE.clone()),
            }),
            _ => Err(Error::new(Errno::ENOENT)),
        }
    }

    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(1),
            mode: FileMode::new(S_IFDIR | 0o755),
        })
    }
}

/// The `/dev/null` file.
struct NullFile {}
impl NullFile {
    pub fn new() -> NullFile {
        NullFile {}
    }
}

impl FileLike for NullFile {
    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
}

/// The `/dev/console` file.
struct ConsoleFile {}
impl ConsoleFile {
    pub fn new() -> ConsoleFile {
        ConsoleFile {}
    }
}

impl FileLike for ConsoleFile {
    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        print_str(b"\x1b[1m");
        print_str(buf);
        print_str(b"\x1b[0m");
        Ok(buf.len())
    }
}

pub fn init() {
    DEV_FS.init(|| Arc::new(DevFs::new()));
}

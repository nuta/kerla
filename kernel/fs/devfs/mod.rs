use self::{null::NullFile, tty::Tty};

use crate::{
    fs::{
        file_system::FileSystem,
        inode::{DirEntry, Directory, FileLike, INode, INodeNo},
        stat::{FileMode, Stat, S_IFDIR},
    },
    result::{Errno, Error, Result},
};
use alloc::sync::Arc;
use penguin_utils::once::Once;

mod null;
mod tty;

static ROOT_DIR: Once<Arc<dyn Directory>> = Once::new();
static NULL_FILE: Once<Arc<dyn FileLike>> = Once::new();
pub static SERIAL_TTY: Once<Arc<Tty>> = Once::new();

pub static DEV_FS: Once<Arc<DevFs>> = Once::new();

pub struct DevFs {}

impl DevFs {
    pub fn new() -> DevFs {
        ROOT_DIR.init(|| Arc::new(DevRootDir::new()) as Arc<dyn Directory>);
        NULL_FILE.init(|| Arc::new(NullFile::new()) as Arc<dyn FileLike>);
        SERIAL_TTY.init(|| Arc::new(Tty::new()));
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
    fn lookup(&self, name: &str) -> Result<INode> {
        match name {
            "null" => Ok(INode::FileLike(NULL_FILE.clone())),
            "console" => Ok(INode::FileLike(SERIAL_TTY.clone())),
            _ => Err(Error::new(Errno::ENOENT)),
        }
    }

    fn readdir(&self, _index: usize) -> Result<Option<DirEntry>> {
        todo!()
    }

    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(1),
            mode: FileMode::new(S_IFDIR | 0o755),
            ..Stat::zeroed()
        })
    }

    fn create_file(&self, _name: &str, _mode: FileMode) -> Result<INode> {
        Err(Errno::ENOSYS.into())
    }

    fn create_dir(&self, _name: &str, _mode: FileMode) -> Result<INode> {
        Err(Errno::ENOSYS.into())
    }
}

pub fn init() {
    DEV_FS.init(|| Arc::new(DevFs::new()));
}

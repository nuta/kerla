use self::{null::NullFile, tty::Tty};

use crate::{
    fs::{
        file_system::FileSystem,
        inode::{DirEntry, Directory, FileLike, INode, INodeNo},
        stat::{FileMode, Stat, S_IFDIR},
    },
    result::{Errno, Error, Result},
};
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::sync::Arc;
use hashbrown::HashMap;
use penguin_utils::once::Once;

use super::inode::FileType;

mod null;
mod tty;

static ROOT_DIR: Once<Arc<dyn Directory>> = Once::new();
static NULL_FILE: Once<Arc<dyn FileLike>> = Once::new();
pub static SERIAL_TTY: Once<Arc<Tty>> = Once::new();

pub static DEV_FS: Once<Arc<DevFs>> = Once::new();

pub struct DevFs {}

impl DevFs {
    pub fn new() -> DevFs {
        NULL_FILE.init(|| Arc::new(NullFile::new()) as Arc<dyn FileLike>);
        SERIAL_TTY.init(|| Arc::new(Tty::new()));

        let mut root_dir = DevRootDir::new();
        root_dir.add_file("null", NULL_FILE.clone());
        root_dir.add_file("console", SERIAL_TTY.clone());
        ROOT_DIR.init(|| Arc::new(root_dir) as Arc<dyn Directory>);
        DevFs {}
    }
}

impl FileSystem for DevFs {
    fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        Ok(ROOT_DIR.clone())
    }
}

/// The `/dev` directory.
struct DevRootDir {
    files: HashMap<String, Arc<dyn FileLike>>,
}

impl DevRootDir {
    pub fn new() -> DevRootDir {
        DevRootDir {
            files: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, name: &str, file: Arc<dyn FileLike>) {
        self.files.insert(name.to_owned(), file);
    }
}

impl Directory for DevRootDir {
    fn link(&self, _name: &str, _link_to: &INode) -> Result<()> {
        Err(Error::new(Errno::ENOSYS))
    }

    fn lookup(&self, name: &str) -> Result<INode> {
        self.files
            .get(name)
            .cloned()
            .map(Into::into)
            .ok_or_else(|| Error::new(Errno::ENOENT))
    }

    fn readdir(&self, index: usize) -> Result<Option<DirEntry>> {
        let (name, file) = match self.files.iter().nth(index) {
            Some((name, file)) => (name, file),
            None => return Ok(None),
        };

        let entry = DirEntry {
            inode_no: file.stat()?.inode_no,
            file_type: FileType::Regular,
            name: name.to_owned(),
        };

        Ok(Some(entry))
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

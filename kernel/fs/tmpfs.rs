use super::{
    file_system::FileSystem,
    inode::{DirEntry, Directory, FileLike, FileType, INode, INodeNo},
    stat::{FileMode, Stat, S_IFDIR, S_IFREG},
};
use crate::{
    arch::SpinLock,
    result::{Errno, Error, Result},
    user_buffer::UserBuffer,
    user_buffer::UserBufferMut,
};
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use hashbrown::HashMap;
use penguin_utils::once::Once;

pub static TMP_FS: Once<Arc<TmpFs>> = Once::new();

pub struct TmpFs {
    root_dir: Arc<Dir>,
}

impl TmpFs {
    pub fn new() -> TmpFs {
        TmpFs {
            root_dir: Arc::new(Dir::new("".to_owned(), INodeNo::new(1))),
        }
    }
}

impl FileSystem for TmpFs {
    fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        Ok(self.root_dir.clone())
    }
}

struct Dir {
    name: String,
    files: HashMap<String, TmpFsINode>,
    stat: Stat,
}

impl Dir {
    pub fn new(name: String, inode_no: INodeNo) -> Dir {
        Dir {
            name,
            files: HashMap::new(),
            stat: Stat {
                inode_no,
                mode: FileMode::new(S_IFDIR | 0o755),
                ..Stat::zeroed()
            },
        }
    }
}

impl Directory for Dir {
    fn lookup(&self, name: &str) -> Result<INode> {
        self.files
            .get(name)
            .map(|tmpfs_inode| match tmpfs_inode {
                TmpFsINode::File(file) => (file.clone() as Arc<dyn FileLike>).into(),
                TmpFsINode::Directory(dir) => (dir.clone() as Arc<dyn Directory>).into(),
            })
            .ok_or_else(|| Error::new(Errno::ENOENT))
    }

    fn readdir(&self, index: usize) -> Result<Option<DirEntry>> {
        let entry = self.files.values().nth(index).map(|entry| match entry {
            TmpFsINode::Directory(dir) => DirEntry {
                inode_no: dir.stat.inode_no,
                file_type: FileType::Directory,
                name: dir.name.clone(),
            },
            TmpFsINode::File(file) => DirEntry {
                inode_no: file.stat.inode_no,
                file_type: FileType::Regular,
                name: file.name.clone(),
            },
        });

        Ok(entry)
    }

    fn stat(&self) -> Result<Stat> {
        Ok(self.stat)
    }
}

struct File {
    name: String,
    data: SpinLock<Vec<u8>>,
    stat: Stat,
}

impl File {
    pub fn new(name: String, inode_no: INodeNo) -> File {
        File {
            name,
            data: SpinLock::new(Vec::new()),
            stat: Stat {
                inode_no,
                mode: FileMode::new(S_IFREG | 0o644),
                ..Stat::zeroed()
            },
        }
    }

    fn stat(&self) -> Result<Stat> {
        Ok(self.stat)
    }
}

impl FileLike for File {
    fn stat(&self) -> Result<Stat> {
        Ok(self.stat)
    }

    fn read(&self, offset: usize, mut buf: UserBufferMut<'_>) -> Result<usize> {
        // FIXME: What if the offset is beyond data?
        buf.write_bytes(&self.data.lock()[offset..])
    }

    fn write(&self, offset: usize, mut buf: UserBuffer<'_>) -> Result<usize> {
        let mut data = self.data.lock();
        data.resize(offset + buf.remaining_len(), 0);
        buf.read_bytes(&mut data[offset..])
    }
}

enum TmpFsINode {
    File(Arc<File>),
    Directory(Arc<Dir>),
}

pub fn init() {
    TMP_FS.init(|| Arc::new(TmpFs::new()));
}

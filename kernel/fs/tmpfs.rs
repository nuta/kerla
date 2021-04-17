use core::sync::atomic::{AtomicUsize, Ordering};

use super::{
    file_system::FileSystem,
    inode::{DirEntry, Directory, FileLike, FileType, INode, INodeNo},
    opened_file::OpenOptions,
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

fn alloc_inode_no() -> INodeNo {
    // Inode #1 is reserved for the root dir.
    static NEXT_INODE_NO: AtomicUsize = AtomicUsize::new(2);

    INodeNo::new(NEXT_INODE_NO.fetch_add(1, Ordering::SeqCst))
}

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

struct DirInner {
    name: String,
    files: HashMap<String, TmpFsINode>,
    stat: Stat,
}

struct Dir(SpinLock<DirInner>);

impl Dir {
    pub fn new(name: String, inode_no: INodeNo) -> Dir {
        Dir(SpinLock::new(DirInner {
            name,
            files: HashMap::new(),
            stat: Stat {
                inode_no,
                mode: FileMode::new(S_IFDIR | 0o755),
                ..Stat::zeroed()
            },
        }))
    }
}

impl Directory for Dir {
    fn lookup(&self, name: &str) -> Result<INode> {
        self.0
            .lock()
            .files
            .get(name)
            .map(|tmpfs_inode| match tmpfs_inode {
                TmpFsINode::File(file) => (file.clone() as Arc<dyn FileLike>).into(),
                TmpFsINode::Directory(dir) => (dir.clone() as Arc<dyn Directory>).into(),
            })
            .ok_or_else(|| Error::new(Errno::ENOENT))
    }

    fn readdir(&self, index: usize) -> Result<Option<DirEntry>> {
        let entry = self
            .0
            .lock()
            .files
            .values()
            .nth(index)
            .map(|entry| match entry {
                TmpFsINode::Directory(dir) => {
                    let dir = dir.0.lock();
                    DirEntry {
                        inode_no: dir.stat.inode_no,
                        file_type: FileType::Directory,
                        name: dir.name.clone(),
                    }
                }
                TmpFsINode::File(file) => DirEntry {
                    inode_no: file.stat.inode_no,
                    file_type: FileType::Regular,
                    name: file.name.clone(),
                },
            });

        Ok(entry)
    }

    fn stat(&self) -> Result<Stat> {
        Ok(self.0.lock().stat)
    }

    fn create_file(&self, name: &str, _mode: FileMode) -> Result<INode> {
        let mut dir_lock = self.0.lock();
        if dir_lock.files.contains_key(name) {
            return Err(Errno::EEXIST.into());
        }

        let inode = Arc::new(File::new(name.to_owned(), alloc_inode_no()));
        dir_lock
            .files
            .insert(name.to_owned(), TmpFsINode::File(inode.clone()));

        Ok((inode as Arc<dyn FileLike>).into())
    }

    fn create_dir(&self, name: &str, _mode: FileMode) -> Result<INode> {
        let inode = Arc::new(Dir::new(name.to_owned(), alloc_inode_no()));
        self.0
            .lock()
            .files
            .insert(name.to_owned(), TmpFsINode::Directory(inode.clone()));

        Ok((inode as Arc<dyn Directory>).into())
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
}

impl FileLike for File {
    fn stat(&self) -> Result<Stat> {
        Ok(self.stat)
    }

    fn read(
        &self,
        offset: usize,
        mut buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        // FIXME: What if the offset is beyond data?
        buf.write_bytes(&self.data.lock()[offset..])
    }

    fn write(
        &self,
        offset: usize,
        mut buf: UserBuffer<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
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

use crate::{
    prelude::*,
    user_buffer::{UserBufReader, UserBufWriter},
};
use core::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{
    file_system::FileSystem,
    inode::{DirEntry, Directory, FileLike, FileType, INode, INodeNo},
    opened_file::OpenOptions,
    stat::{FileMode, Stat, S_IFDIR, S_IFREG},
};
use crate::{
    result::{Errno, Error, Result},
    user_buffer::UserBuffer,
    user_buffer::UserBufferMut,
};
use hashbrown::HashMap;
use kerla_runtime::spinlock::SpinLock;
use kerla_utils::{downcast::downcast, once::Once};

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
            root_dir: Arc::new(Dir::new(INodeNo::new(1))),
        }
    }

    pub fn root_tmpfs_dir(&self) -> &Arc<Dir> {
        &self.root_dir
    }
}

impl FileSystem for TmpFs {
    fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        Ok(self.root_dir.clone())
    }
}

enum TmpFsINode {
    File(Arc<dyn FileLike>),
    Directory(Arc<Dir>),
}

struct DirInner {
    files: HashMap<String, TmpFsINode>,
    stat: Stat,
}

pub struct Dir(SpinLock<DirInner>);

impl Dir {
    pub fn new(inode_no: INodeNo) -> Dir {
        Dir(SpinLock::new(DirInner {
            files: HashMap::new(),
            stat: Stat {
                inode_no,
                mode: FileMode::new(S_IFDIR | 0o755),
                ..Stat::zeroed()
            },
        }))
    }

    pub fn add_dir(&self, name: &str) -> Arc<Dir> {
        let dir = Arc::new(Dir::new(alloc_inode_no()));
        self.0
            .lock()
            .files
            .insert(name.to_owned(), TmpFsINode::Directory(dir.clone()));
        dir
    }

    pub fn add_file(&self, name: &str, file: Arc<dyn FileLike>) {
        self.0
            .lock()
            .files
            .insert(name.to_owned(), TmpFsINode::File(file));
    }
}

impl Directory for Dir {
    fn lookup(&self, name: &str) -> Result<INode> {
        self.0
            .lock()
            .files
            .get(name)
            .map(|tmpfs_inode| match tmpfs_inode {
                TmpFsINode::File(file) => file.clone().into(),
                TmpFsINode::Directory(dir) => (dir.clone() as Arc<dyn Directory>).into(),
            })
            .ok_or_else(|| Error::new(Errno::ENOENT))
    }

    fn readdir(&self, index: usize) -> Result<Option<DirEntry>> {
        let dir_lock = self.0.lock();
        let (name, inode) = match dir_lock.files.iter().nth(index) {
            Some(entry) => entry,
            None => {
                return Ok(None);
            }
        };

        let entry = match inode {
            TmpFsINode::Directory(dir) => {
                let dir = dir.0.lock();
                DirEntry {
                    inode_no: dir.stat.inode_no,
                    file_type: FileType::Directory,
                    name: name.clone(),
                }
            }
            TmpFsINode::File(file) => DirEntry {
                inode_no: file.stat()?.inode_no,
                file_type: FileType::Regular,
                name: name.clone(),
            },
        };

        Ok(Some(entry))
    }

    fn stat(&self) -> Result<Stat> {
        Ok(self.0.lock().stat)
    }

    fn link(&self, name: &str, link_to: &INode) -> Result<()> {
        let tmpfs_inode = match link_to {
            INode::FileLike(file_like) => TmpFsINode::File(file_like.clone()),
            INode::Directory(dir) => {
                let dir: &Arc<Dir> = downcast(dir).unwrap();
                TmpFsINode::Directory(dir.clone())
            }
            INode::Symlink(_) => unreachable!(), /* symblic links are not supported yet */
        };

        self.0.lock().files.insert(name.to_owned(), tmpfs_inode);
        Ok(())
    }

    fn create_file(&self, name: &str, _mode: FileMode) -> Result<INode> {
        let mut dir_lock = self.0.lock();
        if dir_lock.files.contains_key(name) {
            return Err(Errno::EEXIST.into());
        }

        let inode = Arc::new(File::new(alloc_inode_no()));
        dir_lock
            .files
            .insert(name.to_owned(), TmpFsINode::File(inode.clone()));

        Ok((inode as Arc<dyn FileLike>).into())
    }

    fn create_dir(&self, name: &str, _mode: FileMode) -> Result<INode> {
        let inode = Arc::new(Dir::new(alloc_inode_no()));
        self.0
            .lock()
            .files
            .insert(name.to_owned(), TmpFsINode::Directory(inode.clone()));

        Ok((inode as Arc<dyn Directory>).into())
    }
}

impl fmt::Debug for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TmpFsDir").finish()
    }
}

struct File {
    data: SpinLock<Vec<u8>>,
    stat: Stat,
}

impl File {
    pub fn new(inode_no: INodeNo) -> File {
        File {
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

    fn read(&self, offset: usize, buf: UserBufferMut<'_>, _options: &OpenOptions) -> Result<usize> {
        let data = self.data.lock();
        if offset > data.len() {
            return Ok(0);
        }

        let mut writer = UserBufWriter::from(buf);
        writer.write_bytes(&data[offset..])
    }

    fn write(&self, offset: usize, buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        let mut data = self.data.lock();
        let mut reader = UserBufReader::from(buf);
        data.resize(offset + reader.remaining_len(), 0);
        reader.read_bytes(&mut data[offset..])
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TmpFsFile").finish()
    }
}

pub fn init() {
    TMP_FS.init(|| Arc::new(TmpFs::new()));
}

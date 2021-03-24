use super::{
    file_system::FileSystem,
    inode::{DirEntry, Directory, FileLike, INode, INodeNo},
    path::PathBuf,
    stat::{FileMode, Stat, S_IFDIR},
};
use crate::{
    arch::{print_str, SpinLock},
    process::WaitQueue,
    result::{Errno, Error, Result},
};
use alloc::{collections::VecDeque, sync::Arc};
use penguin_utils::once::Once;

static ROOT_DIR: Once<Arc<dyn Directory>> = Once::new();
static NULL_FILE: Once<Arc<dyn FileLike>> = Once::new();
pub static CONSOLE_FILE: Once<Arc<ConsoleFile>> = Once::new();

pub static DEV_FS: Once<Arc<DevFs>> = Once::new();

pub struct DevFs {}

impl DevFs {
    pub fn new() -> DevFs {
        ROOT_DIR.init(|| Arc::new(DevRootDir::new()) as Arc<dyn Directory>);
        NULL_FILE.init(|| Arc::new(NullFile::new()) as Arc<dyn FileLike>);
        CONSOLE_FILE.init(|| Arc::new(ConsoleFile::new()));
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
            ..Stat::zeroed()
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

struct ConsoleInner {
    // FIXME: We must not use collections which may allocate a memory in the
    // interrupt context: use something else like ArrayDeque instead.
    input: VecDeque<char>,
}

/// The `/dev/console` file.
pub struct ConsoleFile {
    inner: SpinLock<ConsoleInner>,
    wait_queue: WaitQueue,
}

impl ConsoleFile {
    pub fn new() -> ConsoleFile {
        ConsoleFile {
            wait_queue: WaitQueue::new(),
            inner: SpinLock::new(ConsoleInner {
                input: VecDeque::new(),
            }),
        }
    }

    pub fn input_char(&self, ch: char) {
        self.write(0, &[ch as u8]).ok();
        self.inner.lock().input.push_back(ch);
        self.wait_queue.wake_one();
    }
}

impl FileLike for ConsoleFile {
    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        loop {
            let mut read_len = 0;
            let mut inner = self.inner.lock();
            while let Some(ch) = inner.input.pop_front() {
                buf[read_len] = ch as u8;
                read_len += 1;
            }

            if read_len > 0 {
                return Ok(read_len);
            }

            drop(inner);
            self.wait_queue.sleep();
        }
    }

    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        print_str(buf);
        Ok(buf.len())
    }
}

pub fn init() {
    DEV_FS.init(|| Arc::new(DevFs::new()));
}

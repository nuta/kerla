use super::{opened_file::OpenOptions, path::PathBuf, stat::FileMode};
use crate::ctypes::c_short;
use crate::prelude::*;
use crate::{fs::stat::Stat, user_buffer::UserBufferMut};
use crate::{net::*, user_buffer::UserBuffer};
use bitflags::bitflags;
use penguin_utils::downcast::Downcastable;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct INodeNo(usize);

impl INodeNo {
    pub const fn new(no: usize) -> INodeNo {
        INodeNo(no)
    }

    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

bitflags! {
    pub struct PollStatus: c_short {
        const POLLIN     = 0x001;
        const POLLPRI    = 0x002;
        const POLLOUT    = 0x004;
        const POLLERR    = 0x008;
        const POLLHUP    = 0x010;
        const POLLNVAL   = 0x020;
        const POLLRDNORM = 0x040;
        const POLLRDBAND = 0x080;
        const POLLWRNORM = 0x100;
        const POLLWRBAND = 0x200;
    }
}

pub trait FileLike: Send + Sync + Downcastable {
    fn open(&self, _options: &OpenOptions) -> Result<Option<Arc<dyn FileLike>>> {
        Ok(None)
    }

    fn stat(&self) -> Result<Stat> {
        Err(Error::new(Errno::EBADF))
    }

    fn poll(&self) -> Result<PollStatus> {
        Err(Error::new(Errno::EBADF))
    }

    fn ioctl(&self, _cmd: usize, _arg: usize) -> Result<isize> {
        Err(Error::new(Errno::EBADF))
    }

    fn read(
        &self,
        _offset: usize,
        _buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    fn write(&self, _offset: usize, _buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    fn bind(&self, _sockaddr: SockAddr) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn listen(&self, _backlog: i32) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn getsockname(&self) -> Result<SockAddr> {
        Err(Error::new(Errno::EBADF))
    }

    fn getpeername(&self) -> Result<SockAddr> {
        Err(Error::new(Errno::EBADF))
    }

    fn fsync(&self) -> Result<()> {
        Ok(())
    }

    fn accept(&self, _options: &OpenOptions) -> Result<(Arc<dyn FileLike>, SockAddr)> {
        Err(Error::new(Errno::EBADF))
    }

    fn connect(&self, _sockaddr: SockAddr, _options: &OpenOptions) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn sendto(
        &self,
        _buf: UserBuffer<'_>,
        _sockaddr: Option<SockAddr>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    fn recvfrom(
        &self,
        _buf: UserBufferMut<'_>,
        _flags: RecvFromFlags,
        _options: &OpenOptions,
    ) -> Result<(usize, SockAddr)> {
        Err(Error::new(Errno::EBADF))
    }
}

/// Represents `d_type` in `linux_dirent`. See `getdents64(2)` manual.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
#[non_exhaustive]
pub enum FileType {
    Directory = 4,
    Regular = 8,
    Link = 10,
}

pub struct DirEntry {
    pub inode_no: INodeNo,
    pub file_type: FileType,
    pub name: String,
}

pub trait Directory: Send + Sync + Downcastable {
    fn stat(&self) -> Result<Stat>;
    fn readdir(&self, index: usize) -> Result<Option<DirEntry>>;
    fn lookup(&self, name: &str) -> Result<INode>;
    fn link(&self, _name: &str, _link_to: &INode) -> Result<()>;
    /// Creates a file. Returns `EEXIST` if the it already exists.
    fn create_file(&self, _name: &str, _mode: FileMode) -> Result<INode>;
    /// Creates a directory. Returns `EEXIST` if the it already exists.
    fn create_dir(&self, _name: &str, _mode: FileMode) -> Result<INode>;

    fn fsync(&self) -> Result<()> {
        Ok(())
    }
}

pub trait Symlink: Send + Sync + Downcastable {
    fn stat(&self) -> Result<Stat>;
    fn linked_to(&self) -> Result<PathBuf>;

    fn fsync(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub enum INode {
    FileLike(Arc<dyn FileLike>),
    Directory(Arc<dyn Directory>),
    Symlink(Arc<dyn Symlink>),
}

impl INode {
    pub fn stat(&self) -> Result<Stat> {
        match self {
            INode::FileLike(file) => file.stat(),
            INode::Symlink(file) => file.stat(),
            INode::Directory(dir) => dir.stat(),
        }
    }

    pub fn as_file(&self) -> Result<&Arc<dyn FileLike>> {
        match self {
            INode::FileLike(file) => Ok(file),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    pub fn as_dir(&self) -> Result<&Arc<dyn Directory>> {
        match self {
            INode::Directory(dir) => Ok(dir),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    pub fn is_file(&self) -> bool {
        matches!(self, INode::FileLike(_))
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, INode::Directory(_))
    }

    pub fn fsync(&self) -> Result<()> {
        match self {
            INode::FileLike(file) => file.fsync(),
            INode::Symlink(file) => file.fsync(),
            INode::Directory(dir) => dir.fsync(),
        }
    }
}

impl From<Arc<dyn FileLike>> for INode {
    fn from(file: Arc<dyn FileLike>) -> Self {
        INode::FileLike(file)
    }
}

impl From<Arc<dyn Directory>> for INode {
    fn from(dir: Arc<dyn Directory>) -> Self {
        INode::Directory(dir)
    }
}

impl From<Arc<dyn Symlink>> for INode {
    fn from(symlink: Arc<dyn Symlink>) -> Self {
        INode::Symlink(symlink)
    }
}

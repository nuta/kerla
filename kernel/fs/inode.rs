use super::{opened_file::OpenOptions, path::PathBuf, stat::FileMode};
use crate::ctypes::c_short;
use crate::result::{Errno, Error, Result};
use crate::{fs::stat::Stat, user_buffer::UserBufferMut};
use crate::{net::*, user_buffer::UserBuffer};
use alloc::string::String;
use alloc::sync::Arc;
use bitflags::bitflags;

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
    }
}

pub trait FileLike: Send + Sync {
    fn stat(&self) -> Result<Stat> {
        Err(Error::new(Errno::EBADF))
    }

    fn poll(&self) -> Result<PollStatus> {
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

    fn bind(&self, _endpoint: Endpoint) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

        Err(Error::new(Errno::EBADF))
    }

    fn connect(&self, _endpoint: Endpoint, _options: &OpenOptions) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn sendto(
        &self,
        _buf: UserBuffer<'_>,
        _endpoint: Endpoint,
        _options: &OpenOptions,
    ) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn recvfrom(
        &self,
        _buf: UserBufferMut<'_>,
        _flags: RecvFromFlags,
        _options: &OpenOptions,
    ) -> Result<(usize, Endpoint)> {
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

pub trait Directory: Send + Sync {
    fn stat(&self) -> Result<Stat>;
    fn readdir(&self, index: usize) -> Result<Option<DirEntry>>;
    fn lookup(&self, name: &str) -> Result<INode>;
    fn create_file(&self, _name: &str, _mode: FileMode) -> Result<INode>;
    fn create_dir(&self, _name: &str, _mode: FileMode) -> Result<INode>;
}

pub trait Symlink: Send + Sync {
    fn stat(&self) -> Result<Stat>;
    fn linked_to(&self) -> Result<PathBuf>;
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

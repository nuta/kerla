use crate::result::{Errno, Error, Result};
use crate::{fs::stat::Stat, user_buffer::UserBufferMut};
use crate::{net::*, user_buffer::UserBuffer};
use alloc::string::String;
use alloc::sync::Arc;

use super::path::PathBuf;

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

pub trait FileLike: Send + Sync {
    fn stat(&self) -> Result<Stat> {
        Err(Error::new(Errno::EBADF))
    }

    fn read(&self, _offset: usize, _buf: UserBufferMut<'_>) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    fn write(&self, _offset: usize, _buf: UserBuffer<'_>) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    fn bind(&self, _endpoint: Endpoint) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn connect(&self, _endpoint: Endpoint) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn sendto(&self, _buf: &[u8], _endpoint: Endpoint) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    fn recvfrom(&self, _buf: &mut [u8], _flags: RecvFromFlags) -> Result<(usize, Endpoint)> {
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

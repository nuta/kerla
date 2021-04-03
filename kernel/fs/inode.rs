use crate::fs::stat::Stat;
use crate::net::*;
use crate::result::{Errno, Error, Result};
use alloc::sync::Arc;

use super::path::PathBuf;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct INodeNo(usize);

impl INodeNo {
    pub const fn new(no: usize) -> INodeNo {
        INodeNo(no)
    }
}

pub trait FileLike: Send + Sync {
    fn stat(&self) -> Result<Stat> {
        Err(Error::new(Errno::EBADF))
    }

    fn read(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    fn write(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
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

pub struct DirEntry {
    pub inode: INode,
}

pub trait Directory: Send + Sync {
    fn stat(&self) -> Result<Stat>;
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

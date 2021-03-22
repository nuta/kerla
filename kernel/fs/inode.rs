use crate::fs::stat::Stat;
use crate::result::Result;
use alloc::sync::Arc;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct INodeNo(usize);

impl INodeNo {
    pub const fn new(no: usize) -> INodeNo {
        INodeNo(no)
    }
}

pub trait FileLike: Send + Sync {
    fn stat(&self) -> Result<Stat>;
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize>;
}

pub struct DirEntry {
    pub inode: INode,
}

pub trait Directory: Send + Sync {
    fn stat(&self) -> Result<Stat>;
    fn lookup(&self, name: &str) -> Result<DirEntry>;
}

#[derive(Clone)]
pub enum INode {
    FileLike(Arc<dyn FileLike>),
    Directory(Arc<dyn Directory>),
}

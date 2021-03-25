use super::inode::{FileLike, INode};
use crate::arch::SpinLock;
use crate::result::{Errno, Error, Result};
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;

bitflags! {
    pub struct OpenMode: u32 {
        const O_RDONLY = 0o0;
        const O_WRONLY = 0o1;
        const O_RDWR = 0o2;
    }
}

/// A file descriptor.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Fd(i32);

impl Fd {
    pub const fn new(value: i32) -> Fd {
        Fd(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

pub struct OpenedFile {
    inode: INode,
    pos: usize,
}

impl OpenedFile {
    pub fn new(inode: INode, _mode: OpenMode, pos: usize) -> OpenedFile {
        OpenedFile { inode, pos }
    }

    pub fn as_file(&self) -> Result<&Arc<dyn FileLike>> {
        match &self.inode {
            INode::FileLike(file) => Ok(file),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read_len = self.as_file()?.read(self.pos, buf)?;
        self.pos += read_len;
        Ok(read_len)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let written_len = self.as_file()?.write(self.pos, buf)?;
        self.pos += written_len;
        Ok(written_len)
    }
}

pub struct OpenedFileTable {
    files: Vec<Option<Arc<SpinLock<OpenedFile>>>>,
}

impl OpenedFileTable {
    pub fn new() -> OpenedFileTable {
        OpenedFileTable {
            files: Vec::new(),
        }
    }

    pub fn get(&self, fd: Fd) -> Result<&Arc<SpinLock<OpenedFile>>> {
        match self.files.get(fd.as_usize()) {
            Some(Some(opened_file)) => Ok(opened_file),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    pub fn open_with_fixed_fd(
        &mut self,
        fd: Fd,
        opened_file: Arc<SpinLock<OpenedFile>>,
    ) -> Result<()> {
        match self.files.get_mut(fd.as_usize()) {
            Some(Some(_)) => {
                return Err(Error::with_message(
                    Errno::EBADF,
                    "already opened at the fd",
                ));
            }
            Some(entry @ None) => {
                *entry = Some(opened_file);
            }
            None => {
                // FIXME: Deny too big fd
                self.files.resize(fd.as_usize() + 1, None);
                self.files[fd.as_usize()] = Some(opened_file);
            }
        }

        Ok(())
    }
}

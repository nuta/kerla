use super::inode::{FileLike, INode};
use crate::arch::SpinLock;
use crate::net::*;
use crate::result::{Errno, Error, Result};
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;

const FD_MAX: i32 = 1024;

bitflags! {
    pub struct OpenFlags: i32 {
        const O_CREAT = 0o100;
        const O_TRUNC = 0o1000;
        const O_APPEND = 0o2000;
        const O_CLOEXEC  = 0o2000000;
    }
}

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

    pub fn bind(&mut self, endpoint: Endpoint) -> Result<()> {
        self.as_file()?.bind(endpoint)
    }

    pub fn connect(&mut self, endpoint: Endpoint) -> Result<()> {
        self.as_file()?.connect(endpoint)
    }

    pub fn sendto(&mut self, buf: &[u8], endpoint: Endpoint) -> Result<()> {
        self.as_file()?.sendto(buf, endpoint)
    }

    pub fn recvfrom(&mut self, buf: &mut [u8], flags: RecvFromFlags) -> Result<(usize, Endpoint)> {
        self.as_file()?.recvfrom(buf, flags)
    }
}

#[derive(Clone)]
pub struct OpenedFileTable {
    files: Vec<Option<Arc<SpinLock<OpenedFile>>>>,
    prev_fd: i32,
}

impl OpenedFileTable {
    pub fn new() -> OpenedFileTable {
        OpenedFileTable {
            files: Vec::new(),
            prev_fd: 1,
        }
    }

    pub fn get(&self, fd: Fd) -> Result<&Arc<SpinLock<OpenedFile>>> {
        match self.files.get(fd.as_usize()) {
            Some(Some(opened_file)) => Ok(opened_file),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    pub fn close(&mut self, fd: Fd) -> Result<()> {
        match self.files.get_mut(fd.as_usize()) {
            Some(opened_file) => *opened_file = None,
            _ => return Err(Error::new(Errno::EBADF)),
        }

        Ok(())
    }

    pub fn open(&mut self, inode: INode) -> Result<Fd> {
        let mut i = (self.prev_fd + 1) % FD_MAX;
        while i != self.prev_fd {
            if matches!(self.files.get(i as usize), Some(None) | None) {
                // It looks the fd number is not in use. Open the file at that fd.
                let fd = Fd::new(i);
                self.open_with_fixed_fd(fd, Arc::new(SpinLock::new(OpenedFile { inode, pos: 0 })))?;
                return Ok(fd);
            }

            i = (i + 1) % FD_MAX;
        }

        Err(Error::new(Errno::ENFILE))
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

    pub fn fork(&self) -> OpenedFileTable {
        self.clone()
    }
}

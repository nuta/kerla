use super::inode::{DirEntry, Directory, FileLike, INode};
use crate::ctypes::c_int;
use crate::fs::inode::PollStatus;
use crate::result::{Errno, Error, Result};
use crate::{arch::SpinLock, user_buffer::UserBufferMut};
use crate::{net::*, user_buffer::UserBuffer};
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;

const FD_MAX: c_int = 1024;

bitflags! {
    pub struct OpenFlags: i32 {
        const O_RDONLY = 0o0;
        const O_WRONLY = 0o1;
        const O_RDWR = 0o2;
        const O_CREAT = 0o100;
        const O_TRUNC = 0o1000;
        const O_APPEND = 0o2000;
        const O_NONBLOCK = 0o4000;
        const O_DIRECTORY = 0o200000;
        const O_CLOEXEC  = 0o2000000;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct OpenOptions {
    pub nonblock: bool,
}

impl OpenOptions {
    pub fn readwrite() -> OpenOptions {
        OpenOptions { nonblock: false }
    }
}

impl From<OpenFlags> for OpenOptions {
    fn from(flags: OpenFlags) -> OpenOptions {
        OpenOptions {
            nonblock: flags.contains(OpenFlags::O_NONBLOCK),
        }
    }
}

/// A file descriptor.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Fd(c_int);

impl Fd {
    pub const fn new(value: i32) -> Fd {
        Fd(value)
    }

    pub const fn as_int(self) -> c_int {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

pub struct OpenedFile {
    inode: INode,
    pos: usize,
    options: OpenOptions,
}

impl OpenedFile {
    pub fn new(inode: INode, options: OpenOptions, pos: usize) -> OpenedFile {
        OpenedFile {
            inode,
            options,
            pos,
        }
    }

    pub fn as_file(&self) -> Result<&Arc<dyn FileLike>> {
        match &self.inode {
            INode::FileLike(file) => Ok(file),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    pub fn as_dir(&self) -> Result<&Arc<dyn Directory>> {
        match &self.inode {
            INode::Directory(dir) => Ok(dir),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn read(&mut self, buf: UserBufferMut<'_>) -> Result<usize> {
        let read_len = self.as_file()?.read(self.pos, buf, &self.options)?;
        self.pos += read_len;
        Ok(read_len)
    }

    pub fn write(&mut self, buf: UserBuffer<'_>) -> Result<usize> {
        let written_len = self.as_file()?.write(self.pos, buf, &self.options)?;
        self.pos += written_len;
        Ok(written_len)
    }

    pub fn listen(&mut self, backlog: i32) -> Result<()> {
        self.as_file()?.listen(backlog)
    }

    pub fn accept(&mut self) -> Result<(Arc<dyn FileLike>, Endpoint)> {
        self.as_file()?.accept(&self.options)
    }

    pub fn bind(&mut self, endpoint: Endpoint) -> Result<()> {
        self.as_file()?.bind(endpoint)
    }

    pub fn connect(&mut self, endpoint: Endpoint) -> Result<()> {
        self.as_file()?.connect(endpoint, &self.options)
    }

    pub fn sendto(&mut self, buf: UserBuffer<'_>, endpoint: Endpoint) -> Result<()> {
        self.as_file()?.sendto(buf, endpoint, &self.options)
    }

    pub fn recvfrom(
        &mut self,
        buf: UserBufferMut<'_>,
        flags: RecvFromFlags,
    ) -> Result<(usize, Endpoint)> {
        self.as_file()?.recvfrom(buf, flags, &self.options)
    }

    pub fn poll(&mut self) -> Result<PollStatus> {
        self.as_file()?.poll()
    }

    pub fn readdir(&mut self) -> Result<Option<DirEntry>> {
        let entry = self.as_dir()?.readdir(self.pos)?;
        self.pos += 1;
        Ok(entry)
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
            _ => return Err(Errno::EBADF.into()),
        }

        Ok(())
    }

    pub fn open(&mut self, inode: INode, options: OpenOptions) -> Result<Fd> {
        let mut i = (self.prev_fd + 1) % FD_MAX;
        while i != self.prev_fd {
            if matches!(self.files.get(i as usize), Some(None) | None) {
                // It looks the fd number is not in use. Open the file at that fd.
                let fd = Fd::new(i);
                self.open_with_fixed_fd(
                    fd,
                    Arc::new(SpinLock::new(OpenedFile {
                        inode,
                        options,
                        pos: 0,
                    })),
                )?;
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
            None if fd.as_int() >= FD_MAX => {
                return Err(Errno::EBADF.into());
            }
            None => {
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

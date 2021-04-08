use super::inode::{DirEntry, Directory, FileLike, INode};
use crate::alloc::borrow::ToOwned;
use crate::ctypes::c_int;
use crate::fs::inode::PollStatus;
use crate::result::{Errno, Error, Result};
use crate::{arch::SpinLock, user_buffer::UserBufferMut};
use crate::{net::*, user_buffer::UserBuffer};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
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

/// Represents a path component.
///
/// For example, in `/tmp/foo.txt`, `tmp` and `foo.txt` have separate `PathComponent`
/// instances.
#[derive(Clone)]
pub struct PathComponent {
    /// The parent directory. `None` if this is the root directory.
    pub parent_dir: Option<Arc<PathComponent>>,
    /// THe component name (e.g. `tmp` or `foo.txt` in `/tmp/foo.txt`).
    pub name: String,
    /// The referenced inode.
    pub inode: INode,
}

pub static PATH_COMPONENT_TABLE: SpinLock<BTreeMap<(usize, String), Weak<PathComponent>>> =
    SpinLock::new(BTreeMap::new());

pub fn resolve_path_component<F>(
    parent_dir: &Arc<PathComponent>,
    name: &str,
    inode_resolver: F,
) -> Result<Arc<PathComponent>>
where
    F: FnOnce(&Arc<PathComponent>, &str) -> Result<INode>,
{
    let parent_ptr = Arc::as_ptr(parent_dir) as usize;

    // FIXME: Don't copy `name` into a String until we actually need it.
    let key = (parent_ptr, name.to_owned());

    let mut table = PATH_COMPONENT_TABLE.lock();
    if let Some(existing) = table.get(&key).and_then(|weak| weak.upgrade()) {
        Ok(existing)
    } else {
        let inode = inode_resolver(parent_dir, name)?;
        let new_path_comp = Arc::new(PathComponent {
            name: name.to_owned(),
            inode,
            parent_dir: Some(parent_dir.clone()),
        });
        table.insert(key, Arc::downgrade(&new_path_comp));
        Ok(new_path_comp)
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
            pos,
            options,
        }
    }

    pub fn as_file(&self) -> Result<&Arc<dyn FileLike>> {
        self.inode.as_file()
    }

    pub fn as_dir(&self) -> Result<&Arc<dyn Directory>> {
        self.inode.as_dir()
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

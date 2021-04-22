use super::{
    inode::{DirEntry, Directory, FileLike, INode},
    path::PathBuf,
};
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
        const O_EXCL = 0o200;
        const O_NOCTTY = 0o400; // TODO:
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
    pub close_on_exec: bool,
}

impl OpenOptions {
    pub fn new(nonblock: bool, cloexec: bool) -> OpenOptions {
        OpenOptions {
            nonblock,
            close_on_exec: cloexec,
        }
    }

    pub fn empty() -> OpenOptions {
        OpenOptions {
            nonblock: false,
            close_on_exec: false,
        }
    }

    pub fn readwrite() -> OpenOptions {
        OpenOptions {
            nonblock: false,
            close_on_exec: false,
        }
    }
}

impl From<OpenFlags> for OpenOptions {
    fn from(flags: OpenFlags) -> OpenOptions {
        OpenOptions {
            nonblock: flags.contains(OpenFlags::O_NONBLOCK),
            close_on_exec: flags.contains(OpenFlags::O_CLOEXEC),
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

impl PathComponent {
    pub fn new_anonymous(inode: INode) -> Arc<PathComponent> {
        Arc::new(PathComponent {
            parent_dir: None,
            name: "anon".to_owned(),
            inode,
        })
    }

    pub fn resolve_absolute_path(&self) -> PathBuf {
        let path = if self.parent_dir.is_some() {
            let mut path = String::from(&self.name);
            let mut parent_dir = &self.parent_dir;
            while let Some(path_comp) = parent_dir {
                path = path_comp.name.clone() + "/" + &path;
                parent_dir = &path_comp.parent_dir;
            }

            // The last parent_dir is the root directory and its name is empty. Thus,
            // the computed path must be an absolute path.
            debug_assert!(path.starts_with('/'));
            path
        } else {
            // `self` points to the root directory.
            "/".to_owned()
        };

        PathBuf::from(path)
    }
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
    path: Arc<PathComponent>,
    pos: usize,
    options: OpenOptions,
}

impl OpenedFile {
    pub fn new(path: Arc<PathComponent>, options: OpenOptions, pos: usize) -> OpenedFile {
        OpenedFile { path, pos, options }
    }

    pub fn as_file(&self) -> Result<&Arc<dyn FileLike>> {
        self.path.inode.as_file()
    }

    pub fn as_dir(&self) -> Result<&Arc<dyn Directory>> {
        self.path.inode.as_dir()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn path(&self) -> &Arc<PathComponent> {
        &self.path
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

    pub fn set_cloexec(&mut self, cloexec: bool) {
        self.options.close_on_exec = cloexec;
    }

    pub fn set_flags(&mut self, flags: OpenFlags) -> Result<()> {
        if flags.contains(OpenFlags::O_NONBLOCK) {
            self.options.nonblock = true;
        }

        Ok(())
    }

    pub fn fsync(&mut self) -> Result<()> {
        self.path.inode.fsync()
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
struct LocalOpenedFile {
    opened_file: Arc<SpinLock<OpenedFile>>,
    close_on_exec: bool,
}

#[derive(Clone)]
pub struct OpenedFileTable {
    files: Vec<Option<LocalOpenedFile>>,
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
            Some(Some(LocalOpenedFile { opened_file, .. })) => Ok(opened_file),
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

    pub fn open(&mut self, path: Arc<PathComponent>, options: OpenOptions) -> Result<Fd> {
        self.alloc_fd(None).and_then(|fd| {
            self.open_with_fixed_fd(
                fd,
                Arc::new(SpinLock::new(OpenedFile {
                    path,
                    options,
                    pos: 0,
                })),
                options,
            )
            .and_then(|_| Ok(fd))
        })
    }

    pub fn open_with_fixed_fd(
        &mut self,
        fd: Fd,
        opened_file: Arc<SpinLock<OpenedFile>>,
        options: OpenOptions,
    ) -> Result<()> {
        match self.files.get_mut(fd.as_usize()) {
            Some(Some(_)) => {
                return Err(Error::with_message(
                    Errno::EBADF,
                    "already opened at the fd",
                ));
            }
            Some(entry @ None) => {
                *entry = Some(LocalOpenedFile {
                    opened_file: opened_file.clone(),
                    close_on_exec: options.close_on_exec,
                });
            }
            None if fd.as_int() >= FD_MAX => {
                return Err(Errno::EBADF.into());
            }
            None => {
                self.files.resize(fd.as_usize() + 1, None);
                self.files[fd.as_usize()] = Some(LocalOpenedFile {
                    opened_file: opened_file.clone(),
                    close_on_exec: options.close_on_exec,
                });
            }
        }

        let mut opened_file = opened_file.lock();
        match opened_file.path.inode {
            INode::FileLike(ref file) => {
                // TODO: Explore better approaches on opening a file.
                if let Some(new_file) = file.open(&options)? {
                    opened_file.path = PathComponent::new_anonymous(new_file.into());
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn dup(&mut self, fd: Fd, gte: Option<i32>, options: OpenOptions) -> Result<Fd> {
        let opened_file = match self.files.get(fd.as_usize()) {
            Some(Some(opened_file)) => opened_file.opened_file.clone(),
            _ => return Err(Errno::EBADF.into()),
        };

        self.alloc_fd(gte).and_then(|fd| {
            self.open_with_fixed_fd(fd, opened_file, options)
                .and_then(|_| Ok(fd))
        })
    }

    pub fn dup2(&mut self, old: Fd, new: Fd, options: OpenOptions) -> Result<()> {
        let opened_file = match self.files.get(old.as_usize()) {
            Some(Some(opened_file)) => opened_file.opened_file.clone(),
            _ => return Err(Errno::EBADF.into()),
        };

        if let Some(Some(_)) = self.files.get(new.as_usize()) {
            self.close(new).ok();
        }

        self.open_with_fixed_fd(new, opened_file, options)?;
        Ok(())
    }

    pub fn fork(&self) -> OpenedFileTable {
        self.clone()
    }

    pub fn close_cloexec_files(&mut self) {
        for slot in &mut self.files {
            if matches!(
                slot,
                Some(LocalOpenedFile {
                    close_on_exec: true,
                    ..
                })
            ) {
                *slot = None;
            }
        }
    }

    /// Allocates an unused fd. Note that this method does not any reservations
    /// for the fd: the caller must register it before unlocking this table.
    fn alloc_fd(&mut self, gte: Option<i32>) -> Result<Fd> {
        let (mut i, gte) = match gte {
            Some(gte) => (gte, gte),
            None => ((self.prev_fd + 1) % FD_MAX, 0),
        };

        while i != self.prev_fd && i >= gte {
            if matches!(self.files.get(i as usize), Some(None) | None) {
                // It looks the fd number is not in use. Open the file at that fd.
                return Ok(Fd::new(i));
            }

            i = (i + 1) % FD_MAX;
        }

        Err(Error::new(Errno::ENFILE))
    }
}

// Allow the bad bit mask of O_RDONLY
#![allow(clippy::bad_bit_mask)]

use super::{
    inode::{DirEntry, Directory, FileLike, INode},
    path::PathBuf,
};
use crate::ctypes::c_int;
use crate::fs::inode::PollStatus;
use crate::prelude::*;
use crate::user_buffer::UserBufferMut;
use crate::{net::*, user_buffer::UserBuffer};
use atomic_refcell::AtomicRefCell;
use bitflags::bitflags;
use crossbeam::atomic::AtomicCell;

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
/// This is mainly used for resolving relative paths.
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
    /// Creates an anonymous path.
    ///
    /// Sometimes you need to use this to implmenet file-like objects that are
    /// not reachable from the root directory (e.g. unnamed pipes).
    pub fn new_anonymous(inode: INode) -> Arc<PathComponent> {
        Arc::new(PathComponent {
            parent_dir: None,
            name: "anon".to_owned(),
            inode,
        })
    }

    /// Resolves into the absolute path.
    pub fn resolve_absolute_path(&self) -> PathBuf {
        let path = if self.parent_dir.is_some() {
            let mut path = String::from(&self.name);
            let mut parent_dir = &self.parent_dir;
            // Visit its ancestor directories...
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

/// An opened file.
///
/// This instance can be shared with multiple processes because of fork(2).
pub struct OpenedFile {
    path: Arc<PathComponent>,
    pos: AtomicCell<usize>,
    options: AtomicRefCell<OpenOptions>,
}

impl OpenedFile {
    pub fn new(path: Arc<PathComponent>, options: OpenOptions, pos: usize) -> OpenedFile {
        OpenedFile {
            path,
            pos: AtomicCell::new(pos),
            options: AtomicRefCell::new(options),
        }
    }

    pub fn as_file(&self) -> Result<&Arc<dyn FileLike>> {
        self.path.inode.as_file()
    }

    pub fn as_dir(&self) -> Result<&Arc<dyn Directory>> {
        self.path.inode.as_dir()
    }

    pub fn pos(&self) -> usize {
        self.pos.load()
    }

    pub fn options(&self) -> OpenOptions {
        *self.options.borrow()
    }

    pub fn path(&self) -> &Arc<PathComponent> {
        &self.path
    }

    pub fn inode(&self) -> &INode {
        &self.path.inode
    }

    pub fn read(&self, buf: UserBufferMut<'_>) -> Result<usize> {
        // Avoid holding self.options and self.pos locks by copying.
        let options = self.options();
        let pos = self.pos();

        let read_len = self.as_file()?.read(pos, buf, &options)?;
        self.pos.fetch_add(read_len);
        Ok(read_len)
    }

    pub fn write(&self, buf: UserBuffer<'_>) -> Result<usize> {
        // Avoid holding self.options and self.pos locks by copying.
        let options = self.options();
        let pos = self.pos();

        let written_len = self.as_file()?.write(pos, buf, &options)?;
        self.pos.fetch_add(written_len);
        Ok(written_len)
    }

    pub fn set_cloexec(&self, cloexec: bool) {
        // FIXME: Modify LocalOpenedFile as well!
        self.options.borrow_mut().close_on_exec = cloexec;
    }

    pub fn set_flags(&self, flags: OpenFlags) -> Result<()> {
        if flags.contains(OpenFlags::O_NONBLOCK) {
            self.options.borrow_mut().nonblock = true;
        }

        Ok(())
    }

    pub fn fsync(&self) -> Result<()> {
        self.path.inode.fsync()
    }

    pub fn ioctl(&self, cmd: usize, arg: usize) -> Result<isize> {
        self.as_file()?.ioctl(cmd, arg)
    }

    pub fn listen(&self, backlog: i32) -> Result<()> {
        self.as_file()?.listen(backlog)
    }

    pub fn accept(&self) -> Result<(Arc<dyn FileLike>, SockAddr)> {
        // Avoid holding self.options lock by copying.
        let options = self.options();

        self.as_file()?.accept(&options)
    }

    pub fn bind(&self, sockaddr: SockAddr) -> Result<()> {
        self.as_file()?.bind(sockaddr)
    }

    pub fn shutdown(&self, how: ShutdownHow) -> Result<()> {
        self.as_file()?.shutdown(how)
    }

    pub fn getsockname(&self) -> Result<SockAddr> {
        self.as_file()?.getsockname()
    }

    pub fn getpeername(&self) -> Result<SockAddr> {
        self.as_file()?.getpeername()
    }

    pub fn connect(&self, sockaddr: SockAddr) -> Result<()> {
        // Avoid holding self.options lock by copying.
        let options = self.options();

        self.as_file()?.connect(sockaddr, &options)
    }

    pub fn sendto(&self, buf: UserBuffer<'_>, sockaddr: Option<SockAddr>) -> Result<usize> {
        // Avoid holding self.options lock by copying.
        let options = self.options();

        self.as_file()?.sendto(buf, sockaddr, &options)
    }

    pub fn recvfrom(
        &self,
        buf: UserBufferMut<'_>,
        flags: RecvFromFlags,
    ) -> Result<(usize, SockAddr)> {
        // Avoid holding self.options lock by copying.
        let options = self.options();

        self.as_file()?.recvfrom(buf, flags, &options)
    }

    pub fn poll(&self) -> Result<PollStatus> {
        self.as_file()?.poll()
    }

    pub fn readdir(&self) -> Result<Option<DirEntry>> {
        // Avoid holding self.pos lock by copying.
        let pos = self.pos();

        let entry = self.as_dir()?.readdir(pos)?;
        self.pos.fetch_add(1);
        Ok(entry)
    }
}

/// A opened file with process-local fields.
#[derive(Clone)]
struct LocalOpenedFile {
    opened_file: Arc<OpenedFile>,
    close_on_exec: bool,
}

/// The opened file table.
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

    /// Resolves the opened file by the file descriptor.
    pub fn get(&self, fd: Fd) -> Result<&Arc<OpenedFile>> {
        match self.files.get(fd.as_usize()) {
            Some(Some(LocalOpenedFile { opened_file, .. })) => Ok(opened_file),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    /// Closes an opened file.
    pub fn close(&mut self, fd: Fd) -> Result<()> {
        match self.files.get_mut(fd.as_usize()) {
            Some(opened_file) => *opened_file = None,
            _ => return Err(Errno::EBADF.into()),
        }

        Ok(())
    }

    /// Opens a file.
    pub fn open(&mut self, path: Arc<PathComponent>, options: OpenOptions) -> Result<Fd> {
        self.alloc_fd(None).and_then(|fd| {
            self.open_with_fixed_fd(
                fd,
                Arc::new(OpenedFile {
                    path,
                    options: AtomicRefCell::new(options),
                    pos: AtomicCell::new(0),
                }),
                options,
            )
            .map(|_| fd)
        })
    }

    /// Opens a file with the given file descriptor.
    ///
    /// Returns `EBADF` if the file descriptor is already in use.
    pub fn open_with_fixed_fd(
        &mut self,
        fd: Fd,
        mut opened_file: Arc<OpenedFile>,
        options: OpenOptions,
    ) -> Result<()> {
        if let INode::FileLike(file) = &opened_file.path.inode {
            if let Some(new_inode) = file.open(&options)? {
                // Replace inode if FileLike::open returned Some. Currently it's
                // used only for /dev/ptmx.
                opened_file = Arc::new(OpenedFile {
                    pos: AtomicCell::new(0),
                    options: AtomicRefCell::new(options),
                    path: Arc::new(PathComponent {
                        name: opened_file.path.name.clone(),
                        parent_dir: opened_file.path.parent_dir.clone(),
                        inode: new_inode.into(),
                    }),
                })
            }
        }

        match self.files.get_mut(fd.as_usize()) {
            Some(Some(_)) => {
                return Err(Error::with_message(
                    Errno::EBADF,
                    "already opened at the fd",
                ));
            }
            Some(entry @ None) => {
                *entry = Some(LocalOpenedFile {
                    opened_file,
                    close_on_exec: options.close_on_exec,
                });
            }
            None if fd.as_int() >= FD_MAX => {
                return Err(Errno::EBADF.into());
            }
            None => {
                self.files.resize(fd.as_usize() + 1, None);
                self.files[fd.as_usize()] = Some(LocalOpenedFile {
                    opened_file,
                    close_on_exec: options.close_on_exec,
                });
            }
        }

        Ok(())
    }

    /// Duplicates a file descriptor.
    ///
    /// If `gte` is `Some`, a new file descriptor will be greater than or equals
    /// to that value.
    pub fn dup(&mut self, fd: Fd, gte: Option<i32>, options: OpenOptions) -> Result<Fd> {
        let opened_file = match self.files.get(fd.as_usize()) {
            Some(Some(opened_file)) => opened_file.opened_file.clone(),
            _ => return Err(Errno::EBADF.into()),
        };

        self.alloc_fd(gte).and_then(|fd| {
            self.open_with_fixed_fd(fd, opened_file, options)
                .map(|_| fd)
        })
    }

    /// Duplicates a file descriptor into the given file descriptor `new`.
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

    /// Closes all opened files.
    pub fn close_all(&mut self) {
        self.files.clear();
    }

    /// Closes opened files with `CLOEXEC` set.
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

impl Default for OpenedFileTable {
    fn default() -> OpenedFileTable {
        OpenedFileTable::new()
    }
}
